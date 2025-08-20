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
    pub const DEFAULT_JOINS: &str = "JOIN company ON job_post.company_id = company.id LEFT JOIN job_application ON job_post.id = job_application.job_post_id";
    pub const DEFAULT_WHERE: &str = "company.hidden = 0";
    pub const DEFAULT_ORDER: &str = "job_application.date_applied DESC NULLS FIRST, job_application.date_responded DESC, date_posted DESC, date_retrieved DESC";

    pub async fn fetch_all(
        page: i64,
        page_size: i64,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Vec<Self>> {
        // println!("fetch all");
        let offset = (page - 1) * page_size;
        let mut query = sqlx::QueryBuilder::new("SELECT job_post.* FROM job_post");
        query.push(" ");
        query.push(Self::DEFAULT_JOINS);
        query.push(" WHERE ");
        query.push(Self::DEFAULT_WHERE);
        query.push(" ORDER BY ");
        query.push(Self::DEFAULT_ORDER);
        query.push(" LIMIT ");
        query.push_bind(page_size);
        query.push(" OFFSET ");
        query.push_bind(offset);
        query
            .build_query_as()
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_all_count(executor: &sqlx::SqlitePool) -> anyhow::Result<i64> {
        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM job_post");
        query.push(" ");
        query.push(Self::DEFAULT_JOINS);
        query.push(" WHERE ");
        query.push(Self::DEFAULT_WHERE);
        query
            .build_query_scalar()
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    pub fn add_filters(
        mut query: sqlx::QueryBuilder<'_, sqlx::Sqlite>,
        title: String,
        location: String,
        min_yoe: i64,
        max_yoe: i64,
        onsite: bool,
        hybrid: bool,
        remote: bool,
        company_name: String,
    ) -> sqlx::QueryBuilder<'_, sqlx::Sqlite> {
        // company.name
        if !(company_name).is_empty() {
            query.push(" AND company.name LIKE ");
            query.push_bind(format!("%{}%", company_name.clone()));
        }
        // years of experience
        if !(min_yoe == max_yoe && max_yoe == 0) {
            query.push(" AND min_yoe = ").push_bind(min_yoe);
            if let Some(max_yoe) = (max_yoe > 0 && max_yoe > min_yoe).then_some(max_yoe) {
                query.push(" AND max_yoe <= ").push_bind(max_yoe);
            }
        }
        // job title
        if !title.is_empty() {
            query
                .push(" AND job_title LIKE ")
                .push_bind(format!("%{}%", title.clone())); // push_bind does the quoting
        }
        // location
        if !location.is_empty() {
            query
                .push(" AND location LIKE ")
                .push_bind(format!("%{}%", location.clone()));
        }

        // loc types
        let mut job_loc_types = Vec::with_capacity(3);
        if onsite {
            job_loc_types.push(JobPostLocationType::Onsite.name());
        }
        if hybrid {
            job_loc_types.push(JobPostLocationType::Hybrid.name());
        }
        if remote {
            job_loc_types.push(JobPostLocationType::Remote.name());
        }
        if !job_loc_types.is_empty() {
            query.push(" AND location_type IN (");
            for (i, loc_type) in job_loc_types.iter().enumerate() {
                if i > 0 {
                    query.push(", ");
                }
                query.push_bind(loc_type.clone());
            }
            query.push(")");
        }
        query
    }

    pub async fn filter(
        page: i64,
        page_size: i64,
        title: String,
        location: String,
        min_yoe: i64,
        max_yoe: i64,
        onsite: bool,
        hybrid: bool,
        remote: bool,
        company_name: String,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Vec<JobPost>> {
        let offset = (page - 1) * page_size;
        let mut query = sqlx::QueryBuilder::new("SELECT job_post.* FROM job_post");
        query.push(" ");
        query.push(Self::DEFAULT_JOINS);
        // WHERE
        query.push(" WHERE ");
        // company.hidden
        query.push(Self::DEFAULT_WHERE);
        query = Self::add_filters(
            query,
            title,
            location,
            min_yoe,
            max_yoe,
            onsite,
            hybrid,
            remote,
            company_name,
        );
        // ORDER BY
        query.push(" ORDER BY ");
        query.push(Self::DEFAULT_ORDER);
        query.push(" LIMIT ");
        query.push_bind(page_size);
        query.push(" OFFSET ");
        query.push_bind(offset);
        // ---
        // println!("{}", query.sql());
        query
            .build_query_as()
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn filter_count(
        title: String,
        location: String,
        min_yoe: i64,
        max_yoe: i64,
        onsite: bool,
        hybrid: bool,
        remote: bool,
        company_name: String,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<i64> {
        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(*) from job_post");
        query.push(" ");
        query.push(Self::DEFAULT_JOINS);
        query.push(" WHERE ");
        query.push(Self::DEFAULT_WHERE);
        query = Self::add_filters(
            query,
            title,
            location,
            min_yoe,
            max_yoe,
            onsite,
            hybrid,
            remote,
            company_name,
        );
        query
            .build_query_scalar()
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn update(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<Self> {
        let posted = self.date_posted.timestamp();
        let updated = sqlx::query_as::<_, Self>(
            r#"UPDATE job_post
                SET
                    location = ?,
                    location_type = ?,
                    url = ?,
                    min_yoe = ?,
                    max_yoe = ?,
                    min_pay_cents = ?,
                    max_pay_cents = ?,
                    date_posted = ?,
                    job_title = ?,
                    benefits = ?,
                    skills = ?,
                    date_retrieved = ?,
                    company_id = ?,
                    apijobs_id = ?
                WHERE id = ?
                RETURNING *
            "#,
        )
        .bind(self.location.clone())
        .bind(self.location_type)
        .bind(self.url.clone())
        .bind(self.min_yoe)
        .bind(self.max_yoe)
        .bind(self.min_pay_cents)
        .bind(self.max_pay_cents)
        .bind(posted)
        .bind(self.job_title.clone())
        .bind(self.benefits.clone())
        .bind(self.skills.clone())
        .bind(self.date_retrieved)
        .bind(self.company_id)
        .bind(self.apijobs_id.clone())
        .bind(self.id)
        .fetch_one(executor)
        .await?;

        Ok(updated)
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
