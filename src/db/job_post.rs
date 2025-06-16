use super::{NullableSqliteDateTime, SqliteDateTime};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, serde::Deserialize, serde::Serialize,
)]
#[sqlx(type_name = "job_post_location_type")]
pub enum JobPostLocationType {
    Onsite,
    Hybrid,
    Remote,
    Unknown,
}

impl JobPostLocationType {
    pub const ALL: [JobPostLocationType; 4] = [
        JobPostLocationType::Onsite,
        JobPostLocationType::Hybrid,
        JobPostLocationType::Remote,
        JobPostLocationType::Unknown,
    ];

    pub fn name(&self) -> String {
        match self {
            JobPostLocationType::Onsite => "Onsite".to_string(),
            JobPostLocationType::Hybrid => "Hybrid".to_string(),
            JobPostLocationType::Remote => "Remote".to_string(),
            JobPostLocationType::Unknown => "Unknown".to_string(),
        }
    }
}

impl std::str::FromStr for JobPostLocationType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Onsite" => Ok(JobPostLocationType::Onsite),
            "Hybrid" => Ok(JobPostLocationType::Hybrid),
            "Remote" => Ok(JobPostLocationType::Remote),
            "Unknown" => Ok(JobPostLocationType::Unknown),
            s => anyhow::bail!("Invalid JobPostLocationType: {s}"),
        }
    }
}

impl From<String> for JobPostLocationType {
    fn from(value: String) -> Self {
        use std::str::FromStr;
        Self::from_str(value.as_str()).expect(&format!(
            "Expected JobPostLocationType, got {value} instead"
        ))
    }
}

impl std::fmt::Display for JobPostLocationType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            JobPostLocationType::Onsite => write!(f, "On-site"),
            JobPostLocationType::Hybrid => write!(f, "Hybrid"),
            JobPostLocationType::Remote => write!(f, "Remote"),
            JobPostLocationType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobPost {
    pub id: i64,
    pub company_id: i64,
    pub location: String,
    pub location_type: JobPostLocationType,
    pub url: String,
    pub min_yoe: Option<i64>,
    pub max_yoe: Option<i64>,
    pub min_pay_cents: Option<i64>,
    pub max_pay_cents: Option<i64>,
    pub date_posted: NullableSqliteDateTime,
    pub date_retrieved: SqliteDateTime,
    pub job_title: String,
    pub benefits: Option<String>,
    pub skills: Option<String>,
    pub pay_unit: Option<String>, // TODO enum
    pub currency: Option<String>,
    pub apijobs_id: Option<String>,
}

impl JobPost {
    pub const DEFAULT_ORDER: &str = "job_application.date_applied DESC NULLS FIRST, job_application.date_responded DESC, date_posted DESC, date_retrieved DESC";

    pub async fn fetch_all(executor: &sqlx::SqlitePool) -> anyhow::Result<Vec<Self>> {
        // println!("fetch all");
        let mut query = sqlx::QueryBuilder::new(
            "SELECT job_post.* FROM job_post JOIN company ON job_post.company_id = company.id
            LEFT JOIN job_application on job_post.id = job_application.job_post_id
            WHERE company.hidden = 0 ORDER BY ",
        );
        query.push(Self::DEFAULT_ORDER);
        query
            .build_query_as()
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn update(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        let posted = self.date_posted.timestamp();
        sqlx::query!(
            r#"UPDATE job_post
                SET
                    location = $1,
                    location_type = $2,
                    url = $3,
                    min_yoe = $4,
                    max_yoe = $5,
                    min_pay_cents = $6,
                    max_pay_cents = $7,
                    date_posted = $8,
                    job_title = $9,
                    benefits = $10,
                    skills = $11,
                    date_retrieved = $12,
                    company_id = $13,
                    apijobs_id = $14
                WHERE id = $15
            "#,
            self.location,
            self.location_type,
            self.url,
            self.min_yoe,
            self.max_yoe,
            self.min_pay_cents,
            self.max_pay_cents,
            posted,
            self.job_title,
            self.benefits,
            self.skills,
            self.date_retrieved,
            self.company_id,
            self.apijobs_id,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn delete(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        // println!("id: {}", id);
        let mut tx = executor.begin().await?;

        sqlx::query!("DELETE FROM job_application WHERE job_post_id = ?", id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                eprintln!("Failed to delete job_application: {}", e);
                e
            })?;

        sqlx::query!("DELETE FROM job_post WHERE id = ?", id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                eprintln!("Failed to delete job_post: {}", e);
                e
            })?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn insert(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO job_post (
                location, location_type, url,
                min_yoe, max_yoe, min_pay_cents,
                max_pay_cents, date_posted, job_title,
                benefits, skills, date_retrieved, company_id, apijobs_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
            self.location,
            self.location_type,
            self.url,
            self.min_yoe,
            self.max_yoe,
            self.min_pay_cents,
            self.max_pay_cents,
            self.date_posted,
            self.job_title,
            self.benefits,
            self.skills,
            self.date_retrieved,
            self.company_id,
            self.apijobs_id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
