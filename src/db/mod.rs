use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqlitePoolOptions, SqliteTypeInfo, SqliteValueRef},
    Database, Decode, Encode, Sqlite, SqlitePool, Type,
};

pub mod company;
pub mod job_application;
pub mod job_post;

pub async fn connect(url: &str) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(100)
        .connect(url)
        .await
        .expect("Failed to open database")
}

pub async fn migrate(acquirable: impl sqlx::Acquire<'_, Database = sqlx::sqlite::Sqlite>) {
    sqlx::migrate!("./migrations")
        .run(acquirable)
        .await
        .expect("Failed to run migrations")
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
}

impl From<Option<i64>> for NullableSqliteDateTime {
    fn from(value: Option<i64>) -> Self {
        let Some(ts) = value else {
            return Self(None);
        };

        let ret = DateTime::from_timestamp(ts, 0).inspect(|_| {
            println!("Failed to convert timestamp to DateTime");
        });

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
