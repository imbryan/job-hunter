mod db;
mod job_hunter;

use clap::Parser;

use db::{bootstrap_sqlx_migrations, connect, migrate};
use job_hunter::JobHunter;

#[derive(Parser)]
pub struct Cli {
    db_path: Option<std::path::PathBuf>,
}

fn main() -> iced::Result {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

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
        .run_with(|| JobHunter::new(conn, handle))
}
