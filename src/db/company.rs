use super::SqliteBoolean;

#[derive(Debug, Clone, Hash, Eq, PartialEq, sqlx::FromRow)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub careers_url: Option<String>,
    pub hidden: SqliteBoolean,
}

impl Company {
    pub async fn fetch_all(executor: &sqlx::SqlitePool) -> anyhow::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT id, name, careers_url, hidden FROM company WHERE hidden = 0 ORDER BY name ASC"
        )
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
        executor: &sqlx::SqlitePool,
    ) -> anyhow::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM company WHERE name LIKE '%' || $1 || '%'",
            name,
        )
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

    pub async fn delete(id: i64, executor: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM company WHERE id = $1", id)
            .execute(executor)
            .await?;

        Ok(())
    }
}

impl std::fmt::Display for Company {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
