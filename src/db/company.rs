use super::SqliteBoolean;
use sqlx::QueryBuilder;

#[derive(Debug, Clone, Hash, Eq, PartialEq, sqlx::FromRow)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub careers_url: Option<String>,
    pub hidden: SqliteBoolean,
}

impl Company {
    pub const DEFAULT_ORDER: &str = "name ASC";

    pub async fn fetch_shown(executor: &sqlx::SqlitePool) -> anyhow::Result<Vec<Self>> {
        let mut query = QueryBuilder::new(
            "SELECT id, name, careers_url, hidden FROM company WHERE hidden = 0 ORDER BY ",
        );
        query.push(Self::DEFAULT_ORDER);
        query
            .build_query_as()
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_one(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<Option<Self>> {
        sqlx::query_as!(Self, "SELECT * FROM company WHERE id = $1", id)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_by_name(
        name: &str,
        include_hidden: bool,
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Vec<Self>> {
        let mut query = QueryBuilder::new("SELECT * FROM company WHERE name LIKE ");
        query.push_bind(format!("%{}%", name));
        if !include_hidden {
            query.push(" AND hidden = 0 ");
        }
        query.push(" ORDER BY ");
        query.push(Self::DEFAULT_ORDER);
        query
            .build_query_as()
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn insert(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO company (name, careers_url, hidden) VALUES ($1, $2, $3)",
            self.name,
            self.careers_url,
            self.hidden,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn update(&self, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE company SET name = $1, careers_url = $2, hidden = $3 WHERE id = $4",
            self.name,
            self.careers_url,
            self.hidden,
            self.id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn hide(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!("UPDATE company SET hidden = 1 WHERE id = $1", id)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn show_all(executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!("UPDATE company SET hidden = 0")
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn solo(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!("UPDATE company SET hidden = 1 WHERE id != $1", id)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn delete(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        let mut tx = executor.begin().await?;

        sqlx::query!(
            "DELETE FROM job_application 
            WHERE job_application.job_post_id IN
            ( 
                SELECT job_post.id FROM job_post WHERE job_post.company_id = ? 
            )",
            id,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!("DELETE FROM job_post WHERE company_id = ?", id)
            .execute(&mut *tx)
            .await?;

        sqlx::query!("DELETE FROM company WHERE id = $1", id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}

impl std::fmt::Display for Company {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
