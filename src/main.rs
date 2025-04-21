mod api;
mod db;
mod job_hunter;

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

use db::{bootstrap_sqlx_migrations, connect, migrate};
use job_hunter::JobHunter;

#[derive(Parser)]
pub struct Cli {
    db_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    apijobs_key: String,
}

fn main() -> iced::Result {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let cfg: AppConfig = {
        let path = std::path::Path::new("config.toml");
        if path.exists() {
            let content = fs::read_to_string(path).expect("Failed to read config");
            toml::from_str(&content).expect("Failed to initiliaze config")
        } else {
            let default = AppConfig {
                apijobs_key: String::new(),
            };
            let toml_str = toml::to_string_pretty(&default).expect("Failed to initiliaze config");
            let mut file = fs::File::create(path).expect("Failed to create config");
            file.write_all(toml_str.as_bytes())
                .expect("Failed to write config");
            default
        }
    };

    let conn = runtime.block_on(async {
        // Get db path argument (mostly for dev purposes)
        let args = Cli::parse();
        let db_path = args.db_path.unwrap_or_else(|| "jobhunter.db".into());

        let db_existed: bool = db_path.exists();

        if !db_existed {
            db::create(db_path.to_str().expect("Invalid database path")).await;
        }

        let conn = connect(db_path.to_str().expect("Invalid database path")).await;
        if db_existed {
            bootstrap_sqlx_migrations(&conn).await;
        }
        migrate(&conn).await;

        conn
    });

    let handle = runtime.handle().clone();

    iced::daemon(JobHunter::title, JobHunter::update, JobHunter::view)
        .theme(JobHunter::theme)
        .subscription(JobHunter::subscription)
        .run_with(|| JobHunter::new(conn, handle, cfg))
}
