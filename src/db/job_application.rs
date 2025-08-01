use iced::advanced::clipboard::Null;

use super::{NullableSqliteDateTime, SqliteBoolean};

#[derive(Debug, Clone, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "job_application_status")]
pub enum JobApplicationStatus {
    New,
    Applied,
    Interview,
    Offer,
    Closed,
    Rejected,
    Withdrawn,
}

impl JobApplicationStatus {
    pub const ALL: [JobApplicationStatus; 7] = [
        JobApplicationStatus::New,
        JobApplicationStatus::Applied,
        JobApplicationStatus::Interview,
        JobApplicationStatus::Offer,
        JobApplicationStatus::Closed,
        JobApplicationStatus::Rejected,
        JobApplicationStatus::Withdrawn,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            JobApplicationStatus::New => "New",
            JobApplicationStatus::Applied => "Applied",
            JobApplicationStatus::Interview => "Interview",
            JobApplicationStatus::Offer => "Offer",
            JobApplicationStatus::Closed => "Closed",
            JobApplicationStatus::Rejected => "Rejected",
            JobApplicationStatus::Withdrawn => "Withdrawn",
        }
    }
}

impl std::str::FromStr for JobApplicationStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "New" => Ok(JobApplicationStatus::New),
            "Applied" => Ok(JobApplicationStatus::Applied),
            "Interview" => Ok(JobApplicationStatus::Interview),
            "Offer" => Ok(JobApplicationStatus::Offer),
            "Closed" => Ok(JobApplicationStatus::Closed),
            "Rejected" => Ok(JobApplicationStatus::Rejected),
            "Withdrawn" => Ok(JobApplicationStatus::Withdrawn),
            _ => Err(()),
        }
    }
}

impl From<String> for JobApplicationStatus {
    fn from(value: String) -> Self {
        use std::str::FromStr;
        Self::from_str(value.as_str()).expect("invalid JobApplicationStatus")
    }
}

impl std::fmt::Display for JobApplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = self.name();
        write!(f, "{}", name)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct JobApplication {
    pub id: i64,
    pub job_post_id: i64,
    pub status: JobApplicationStatus,
    pub date_applied: NullableSqliteDateTime,
    pub date_responded: NullableSqliteDateTime,
    pub interviewed: SqliteBoolean,
}

impl JobApplication {
    pub fn new(
        app_id: i64,
        job_post_id: i64,
        status: JobApplicationStatus,
        date_applied: Option<iced_aw::date_picker::Date>,
        date_responded: Option<iced_aw::date_picker::Date>,
        interviewed: bool,
    ) -> Self {
        Self {
            id: app_id as i64,
            job_post_id: job_post_id as i64,
            status,
            date_applied: NullableSqliteDateTime::from(date_applied),
            date_responded: NullableSqliteDateTime::from(date_responded),
            interviewed: SqliteBoolean(interviewed),
        }
    }

    pub async fn fetch_one(
        application_id: i64,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Option<Self>> {
        let ret = sqlx::query_as!(
            Self,
            r#"SELECT * FROM job_application WHERE id = $1"#,
            application_id,
        )
        .fetch_optional(executor)
        .await?;

        Ok(ret)
    }

    pub async fn fetch_one_by_job_post_id(
        job_post_id: i64,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Option<Self>> {
        let ret = sqlx::query_as!(
            Self,
            r#"SELECT * FROM job_application WHERE job_post_id = $1"#,
            job_post_id,
        )
        .fetch_optional(executor)
        .await?;

        Ok(ret)
    }

    pub async fn insert(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO job_application (status, date_applied, date_responded, job_post_id, interviewed) VALUES ($1, $2, $3, $4, $5)"#,
            self.status,
            self.date_applied,
            self.date_responded,
            self.job_post_id,
            self.interviewed,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn update(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!(
            r#"UPDATE job_application SET status = $1, date_applied = $2, date_responded = $3, interviewed = $4 WHERE id = $5"#,
            self.status,
            self.date_applied,
            self.date_responded,
            self.interviewed,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
