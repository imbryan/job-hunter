use std::fmt::Display;
use std::str::FromStr;

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
    pub min_yoe: Option<i32>,
    pub max_yoe: Option<i32>,
    pub min_pay_cents: Option<i64>,
    pub max_pay_cents: Option<i64>,
    pub date_posted: Option<DateTime<Utc>>,
    pub date_retrieved: DateTime<Utc>,
    pub job_title: String,
}

impl JobPost {
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let sql = "SELECT id, company_id, location, location_type, url,
            min_yoe, max_yoe, min_pay_cents, max_pay_cents, 
            date_posted, date_retrieved, job_title FROM job_post";
        conn.prepare(sql)?
            .query_map([], |row| {
                let location_type_str: String = row.get(3)?;
                let location_type = match JobPostLocationType::from_str(&location_type_str) {
                    Ok(variant) => variant,
                    Err(_) => panic!(),
                };
                let date_posted_timestamp = DateTime::from_timestamp(row.get(9)?, 0);
                let date_retrieved_timestamp = DateTime::from_timestamp(row.get(10)?, 0).unwrap();

                Ok(JobPost {
                    id: row.get(0)?,
                    company_id: row.get(1)?,
                    location: row.get(2)?,
                    location_type: location_type,
                    url: row.get(4)?,
                    min_yoe: row.get(5)?,
                    max_yoe: row.get(6)?,
                    min_pay_cents: row.get(7)?,
                    max_pay_cents: row.get(8)?,
                    date_posted: date_posted_timestamp,
                    date_retrieved: date_retrieved_timestamp,
                    job_title: row.get(11)?,
                })
            })?
            .collect()
    }
}

#[derive(Debug)]
pub enum JobPostLocationType {
    Onsite,
    Hybrid,
    Remote,
}

impl FromStr for JobPostLocationType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Onsite" => Ok(JobPostLocationType::Onsite),
            "Hybrid" => Ok(JobPostLocationType::Hybrid),
            "Remote" => Ok(JobPostLocationType::Remote),
            _ => Err(()),
        }
    }
}

impl Display for JobPostLocationType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            JobPostLocationType::Onsite => write!(f, "On-site"),
            JobPostLocationType::Hybrid => write!(f, "Hybrid"),
            JobPostLocationType::Remote => write!(f, "Remote"),
        }
    }
}

#[derive(Debug)]
pub struct JobApplication {
    pub id: i32,
    pub job_post_id: i32,
    pub status: JobApplicationStatus,
    pub date_applied: Option<DateTime<Utc>>,
    pub date_responded: Option<DateTime<Utc>>,
}

impl JobApplication {
    pub fn get(conn: &Connection, id: i32) -> rusqlite::Result<Self> {
        let sql = "SELECT * FROM job_application WHERE id = ?";
        conn.prepare(sql)?
            .query_row([id], |row| {
                let status_str: String = row.get(2)?;
                let status = match JobApplicationStatus::from_str(&status_str) {
                    Ok(variant) => variant,
                    Err(_) => panic!(),
                };
                
                let date_applied_timestamp = DateTime::from_timestamp(row.get(3)?, 0);
                let date_responded_timestamp = DateTime::from_timestamp(row.get(4)?, 0);

                Ok(JobApplication {
                    id: row.get(0)?,
                    job_post_id: row.get(1)?,
                    status: status,
                    date_applied: date_applied_timestamp,
                    date_responded: date_responded_timestamp,
                })
            })
    }
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

impl FromStr for JobApplicationStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "New" => Ok(JobApplicationStatus::New),
            "Applied" => Ok(JobApplicationStatus::Applied),
            "Interview" => Ok(JobApplicationStatus::Interview),
            "Offer" => Ok(JobApplicationStatus::Offer),
            "Closed" => Ok(JobApplicationStatus::Closed),
            "Rejected" => Ok(JobApplicationStatus::Rejected),
            _ => Err(()),
        }
    }
}

impl Display for JobApplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            JobApplicationStatus::New => write!(f, "New"),
            JobApplicationStatus::Applied => write!(f, "Applied"),
            JobApplicationStatus::Interview => write!(f, "Interview"),
            JobApplicationStatus::Offer => write!(f, "Offer"),
            JobApplicationStatus::Closed => write!(f, "Closed"),
            JobApplicationStatus::Rejected => write!(f, "Rejected"),
        }
    }
}
