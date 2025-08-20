use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteTypeInfo, SqliteValueRef},
    Database, Decode, Encode, Sqlite, SqlitePool, Type,
};

pub mod company;
pub mod job_application;
pub mod job_post;

/* Database */

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
static LAST_RUSQL_MIGRATION: i64 = 9;

pub async fn create(url: &str) -> SqlitePool {
    SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(url)
            .create_if_missing(true),
    )
    .await
    .expect("Failed to create database")
}

pub async fn connect(url: &str) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(100)
        .connect(url)
        .await
        .expect("Failed to open database")
}

pub async fn bootstrap_sqlx_migrations(pool: &sqlx::SqlitePool) {
    let table_exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if table_exists.is_none() {
        sqlx::query(
            r#"
            CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );
            "#,
        )
        .execute(pool)
        .await;

        println!("_sqlx_migrations table created");
    }

    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if row_count == 0 {
        for migration in MIGRATOR.iter() {
            let migration = migration.clone();
            sqlx::query(
                    "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) VALUES (?, ?, CURRENT_TIMESTAMP, 1, ?, 0)"
                )
                .bind(migration.version)
                .bind(migration.description)
                .bind(migration.checksum.to_vec())
                .execute(pool)
                .await;
            if migration.version >= LAST_RUSQL_MIGRATION {
                break;
            }
        }
        println!("_sqlx_migrations legacy rows populated");
    }
}

pub async fn migrate(acquirable: impl sqlx::Acquire<'_, Database = sqlx::sqlite::Sqlite>) {
    MIGRATOR
        .run(acquirable)
        .await
        .expect("Failed to run migrations")
}

pub async fn shutdown(pool: sqlx::SqlitePool) {
    // closing with an owned pool clone
    pool.close().await;
}

/* SqliteDateTime */

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SqliteDateTime(pub DateTime<Utc>);

impl From<i64> for SqliteDateTime {
    fn from(value: i64) -> Self {
        let ret =
            DateTime::from_timestamp(value, 0).expect("Failed to convert i64 to SqliteDateTime");

        Self(ret)
    }
}

impl Type<Sqlite> for SqliteDateTime {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for SqliteDateTime {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let timestamp: i64 = <i64 as Decode<Sqlite>>::decode(value)?;

        let ret = DateTime::from_timestamp(timestamp, 0)
            .map(Self)
            .ok_or_else(|| {
                sqlx::Error::Decode(format!("Invalid timestamp: {}", timestamp).into())
            })?;

        Ok(ret)
    }
}

impl<'q> Encode<'q, Sqlite> for SqliteDateTime {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let timestamp = self.0.timestamp();
        <i64 as Encode<Sqlite>>::encode(timestamp, buf)
    }
}

/* NullableSqliteDateTime */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NullableSqliteDateTime(pub Option<NaiveDate>);

impl Default for NullableSqliteDateTime {
    fn default() -> Self {
        Self(None)
    }
}

impl NullableSqliteDateTime {
    pub const fn timestamp(&self) -> Option<i64> {
        let Some(date) = self.0 else {
            return None;
        };
        Some(
            chrono::NaiveDateTime::new(date, chrono::NaiveTime::MIN)
                .and_utc()
                .timestamp(),
        )
    }

    pub fn format<'a>(&self, fmt: &'a str) -> String {
        let Some(date) = self.0 else {
            return "".to_string();
        };
        date.format(fmt).to_string()
    }

    pub fn from_iso_str(s: &str) -> Self {
        let dt = DateTime::parse_from_rfc3339(s)
            .expect("Failed to parse iso string")
            .with_timezone(&Utc);
        Self(Some(dt.date_naive()))
    }
}

impl From<Option<i64>> for NullableSqliteDateTime {
    fn from(value: Option<i64>) -> Self {
        let Some(ts) = value else {
            return Self(None);
        };

        let ret = DateTime::from_timestamp(ts, 0);

        Self(ret.as_ref().map(DateTime::date_naive))
    }
}

impl From<Option<iced_aw::date_picker::Date>> for NullableSqliteDateTime {
    fn from(value: Option<iced_aw::date_picker::Date>) -> Self {
        Self(value.map(|v| v.into()))
    }
}

impl From<NullableSqliteDateTime> for Option<iced_aw::date_picker::Date> {
    fn from(value: NullableSqliteDateTime) -> Self {
        value.0.map(|date| date.into())
    }
}

impl Type<Sqlite> for NullableSqliteDateTime {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for NullableSqliteDateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        use sqlx::ValueRef;
        if value.is_null() {
            // db returned NULL
            return Ok(NullableSqliteDateTime(None));
        }

        let timestamp: i64 = <i64 as Decode<Sqlite>>::decode(value)?;
        let ret = DateTime::from_timestamp(timestamp, 0)
            .as_ref()
            .map(DateTime::date_naive)
            .map(Some)
            .map(Self)
            .ok_or_else(|| {
                sqlx::Error::Decode(format!("Invalid timestamp: {}", timestamp).into())
            })?;

        Ok(ret)
    }
}

impl<'q> Encode<'q, Sqlite> for NullableSqliteDateTime {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let Some(date) = self.0 else {
            // NULL
            return <Option<i64> as Encode<Sqlite>>::encode(None, buf);
        };

        let timestamp = chrono::NaiveDateTime::new(date, chrono::NaiveTime::MIN)
            .and_utc()
            .timestamp();
        <i64 as Encode<Sqlite>>::encode(timestamp, buf)
    }
}

/* SqliteBoolean */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SqliteBoolean(pub bool);

impl From<i64> for SqliteBoolean {
    fn from(value: i64) -> Self {
        match value {
            0 => Self(false),
            1 => Self(true),
            _ => panic!("Invalid i64 to bool"),
        }
    }
}

impl From<bool> for SqliteBoolean {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<SqliteBoolean> for bool {
    fn from(value: SqliteBoolean) -> bool {
        value.0
    }
}

impl Type<Sqlite> for SqliteBoolean {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for SqliteBoolean {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value: i64 = <i64 as Decode<Sqlite>>::decode(value)?;
        Ok(Self::from(value))
    }
}

impl<'q> Encode<'q, Sqlite> for SqliteBoolean {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let value = if self.0 { 1i64 } else { 0i64 };
        <i64 as Encode<Sqlite>>::encode(value, buf)
    }
}
