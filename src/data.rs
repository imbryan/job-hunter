use std::fmt::Display;
use std::str::FromStr;

use chrono::{Datelike, DateTime, NaiveDate, Utc};
use include_dir::{include_dir, Dir};
use rusqlite::{Connection, params};
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

/* Data conversion and formatting */

pub fn timestamp_to_utc(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.map(|ts| DateTime::from_timestamp(ts, 0))?
}

pub fn get_utc(date: Option<iced_aw::date_picker::Date>) -> Option<chrono::DateTime<Utc>> {
    date.and_then(|date| {
        let naive_date = NaiveDate::from_ymd_opt(date.year, date.month, date.day)?;
        Some(naive_date.and_hms_opt(0,0,0)?.and_utc())
    })
}

pub fn get_iced_date(date: Option<chrono::DateTime<Utc>>) -> Option<iced_aw::date_picker::Date> {
    date.and_then(|date| {
        Some(iced_aw::date_picker::Date::from_ymd(date.year(), date.month(), date.day()) )
    })
}

pub fn get_pay_i64(s: &str) -> Result<i64, String> {
    if let Ok(num) = s.parse::<f64>() {
        return Ok((num * 100.0).round() as i64);
    }

    Err("Invalid input string".to_string())
}

pub fn get_pay_str(num: Option<i64>) -> String {
    match num {
        Some(num) => format!("{:.2}", num as f64 / 100.0),
        None => "".to_string()
    }
}

pub fn opt_str_from_db(str: Option<String>) -> Option<String> {
    match str {
        Some(str) => {
            if str.is_empty() {
                None
            } else {
                Some(str)
            }
        },
        None => None,
    }
}

pub fn format_comma_separated(str: String) -> String {
    str.split(',')
        .map(|s| {
            let mut chars = s.trim().chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
            .collect::<Vec<String>>()
            .join(", ")
}

/* Data models */

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
                id: row.get("id")?,
                name: row.get("name")?,
                careers_url: row.get::<_, Option<String>>("careers_url")?.unwrap_or_else(|| "".to_string()),
            })
        })?
        .collect()
    }

    pub fn get(conn: &Connection, id: i32) -> rusqlite::Result<Self> {
        let sql = "SELECT * FROM company WHERE id = ?";
        conn.prepare(sql)?
            .query_row([id], |row| {
                Ok(Company {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    careers_url: row.get("careers_url")?,
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

#[derive(Debug, Clone)]
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
    pub benefits: Option<String>,
    pub skills: Option<String>,
}

impl JobPost {
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let sql = "SELECT * FROM job_post";
        conn.prepare(sql)?
            .query_map([], |row| {
                let location_type_str: String = row.get("location_type")?;
                let location_type = match JobPostLocationType::from_str(&location_type_str) {
                    Ok(variant) => variant,
                    Err(_) => panic!(),
                };
                let posted: Option<i64> = row.get("date_posted")?;
                let date_retrieved_timestamp = DateTime::from_timestamp(row.get("date_retrieved")?, 0).unwrap();
                Ok(JobPost {
                    id: row.get("id")?,
                    company_id: row.get("company_id")?,
                    location: row.get("location")?,
                    location_type: location_type,
                    url: row.get("url")?,
                    min_yoe: row.get("min_yoe")?,
                    max_yoe: row.get("max_yoe")?,
                    min_pay_cents: row.get("min_pay_cents")?,
                    max_pay_cents: row.get("max_pay_cents")?,
                    date_posted: timestamp_to_utc(posted),
                    date_retrieved: date_retrieved_timestamp,
                    job_title: row.get("job_title")?,
                    benefits: opt_str_from_db(row.get::<&str, Option<String>>("benefits")?),
                    skills: opt_str_from_db(row.get::<&str, Option<String>>("skills")?),
                })
            })?
            .collect()
    }

    pub fn cascade_applications(conn: &Connection, id: i32) -> rusqlite::Result<()> {
        let sql = "DELETE FROM job_application where job_post_id = ?";
        conn.execute(sql, [id])?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i32) -> rusqlite::Result<()> {
        JobPost::cascade_applications(conn, id).expect("Failed to delete job applications");
        let sql = "DELETE FROM job_post WHERE id = ?";
        conn.execute(sql, [id])?;
        Ok(())
    }

    pub fn update(conn: &Connection, post: Self) -> rusqlite::Result<()> {
        let posted = match post.date_posted {
            Some(date) => Some(date.timestamp()),
            None => None,
        };
        let sql = "UPDATE job_post SET location = ?, location_type = ?,
        url = ?, min_yoe = ?, max_yoe = ?, min_pay_cents = ?, max_pay_cents = ?,
        date_posted = ?, job_title = ?, benefits = ?, skills = ? 
        WHERE id = ?";
        conn.execute(sql, params![
            post.location,
            post.location_type.name(),
            post.url,
            post.min_yoe,
            post.max_yoe,
            post.min_pay_cents,
            post.max_pay_cents,
            posted,
            post.job_title,
            post.benefits,
            post.skills,
            post.id,
        ])?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JobPostLocationType {
    Onsite,
    Hybrid,
    Remote,
}

impl JobPostLocationType {
    pub const ALL: [JobPostLocationType; 3] = [
        JobPostLocationType::Onsite,
        JobPostLocationType::Hybrid,
        JobPostLocationType::Remote,
    ];

    pub fn name(&self) -> String {
        match self {
            JobPostLocationType::Onsite => "Onsite".to_owned(),
            JobPostLocationType::Hybrid => "Hybrid".to_owned(),
            JobPostLocationType::Remote => "Remote".to_owned(),
        }
    }
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
                let status_str: String = row.get("status")?;
                let status = match JobApplicationStatus::from_str(&status_str) {
                    Ok(variant) => variant,
                    Err(_) => panic!(),
                };
                
                let applied: Option<i64> = row.get("date_applied")?;
                let responded: Option<i64> = row.get("date_responded")?;

                Ok(JobApplication {
                    id: row.get("id")?,
                    job_post_id: row.get("job_post_id")?,
                    status: status,
                    date_applied: timestamp_to_utc(applied),
                    date_responded: timestamp_to_utc(responded),
                })
            })
    }

    pub fn create(conn: &Connection, application: Self) -> rusqlite::Result<()> {
        let sql = "INSERT INTO job_application (status, date_applied, date_responded, job_post_id) VALUES (?, ?, ?, ?)";
        let applied = match application.date_applied {
            Some(date) => Some(date.timestamp()),
            None => None,
        };
        let responded = match application.date_responded {
            Some(date) => Some(date.timestamp()),
            None => None,
        };
        conn.execute(sql, params![
            application.status.name(), 
            applied, 
            responded, 
            application.job_post_id,
        ])?;
        Ok(())
    }

    pub fn update(conn: &Connection, application: Self) -> rusqlite::Result<()> {
        let applied = match application.date_applied {
            Some(date) => Some(date.timestamp()),
            None => None
        };
        let responded = match application.date_responded {
            Some(date) => Some(date.timestamp()),
            None => None
        };
        let sql = "UPDATE job_application SET status = ?, date_applied = ?, date_responded = ? WHERE id = ?";
        conn.execute(sql, params![
            application.status.name(),
            applied,
            responded,
            application.id.to_string(),
        ])?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JobApplicationStatus {
    New,
    Applied,
    Interview,
    Offer,
    Closed,
    Rejected,
}

impl JobApplicationStatus {
    pub const ALL: [JobApplicationStatus; 6] = [
        JobApplicationStatus::New,
        JobApplicationStatus::Applied,
        JobApplicationStatus::Interview,
        JobApplicationStatus::Offer,
        JobApplicationStatus::Closed,
        JobApplicationStatus::Rejected,
    ];

    pub fn name(&self) -> String {
        match self {
            JobApplicationStatus::New => "New".to_owned(),
            JobApplicationStatus::Applied => "Applied".to_owned(),
            JobApplicationStatus::Interview => "Interview".to_owned(),
            JobApplicationStatus::Offer => "Offer".to_owned(),
            JobApplicationStatus::Closed => "Closed".to_owned(),
            JobApplicationStatus::Rejected => "Rejected".to_owned(),
        }
    }
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
