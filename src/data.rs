use chrono::{DateTime, Utc};
use include_dir::{include_dir, Dir};
use rusqlite::Connection;
use rusqlite_migration::Migrations;

pub fn connect() -> Connection {
    Connection::open("jobhunter.db").expect("Failed to open database")
}

pub fn migrate(conn: &mut Connection) {
    static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");
    let migrations = Migrations::from_directory(&MIGRATIONS_DIR).unwrap();
    migrations.to_latest(conn).unwrap();
    println!("Migrations are up-to-date.");
}

#[derive(Debug, Clone)]
pub struct Company {
    pub id: i32,
    pub name: String,
    pub careers_url: String,
}

impl Company {
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        conn.prepare("SELECT * FROM company")?
            .query_map([], |row| {
            Ok(Company {
                id: row.get(0)?,
                name: row.get(1)?,
                careers_url: row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "".to_string()),
            })
        })?
        .collect()
    }

    pub fn get(conn: &Connection, id: i32) -> rusqlite::Result<Self> {
        let sql = "SELECT * FROM company WHERE id = ?";
        conn.prepare(sql)?
            .query_row([id], |row| {
                Ok(Company {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    careers_url: row.get(2)?,
                })
            })
    }

    pub fn create(conn: &Connection, name: String, careers_url: String) -> rusqlite::Result<()> {
        let sql = "INSERT INTO company (name, careers_url) VALUES (?, ?)";
        conn.execute(sql, [name, careers_url])?;
        Ok(())
    }

    pub fn update(conn: &Connection, company: Self) -> rusqlite::Result<()> {
        let sql = "UPDATE company SET name = ?, careers_url = ? WHERE id = ?";
        conn.execute(sql, [company.name, company.careers_url, company.id.to_string()])?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i32) -> rusqlite::Result<()> {
        let sql = "DELETE FROM company WHERE id = ?";
        conn.execute(sql, [id])?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CompanyAltName {
    pub id: i32,
    pub company_id: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct JobPost {
    pub id: i32,
    pub company_id: i32,
    pub location: String,
    pub location_type: JobPostLocationType,
    pub url: String,
    pub min_yoe: i32,
    pub max_yoe: i32,
    pub min_pay_cents: i64,
    pub max_pay_cents: i64,
    pub date_posted: DateTime<Utc>,
    pub date_retrieved: DateTime<Utc>,
}

#[derive(Debug)]
pub enum JobPostLocationType {
    Onsite,
    Hybrid,
    Remote,
}

#[derive(Debug)]
pub struct JobApplication {
    pub id: i32,
    pub job_post_id: i32,
    pub status: JobApplicationStatus,
    pub date_applied: DateTime<Utc>,
    pub date_responded: DateTime<Utc>,
}

#[derive(Debug)]
pub enum JobApplicationStatus {
    New,
    Applied,
    Interview,
    Offer,
    Closed,
    Rejected,
}
