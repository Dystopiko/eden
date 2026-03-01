use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

/// Eden timestamps are formatted prescribed from [RFC 3339] or
/// `YYYY-MM-DDTHH:MM:SS.SSS+00:00`.
///
/// [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a [`Timestamp`] object based on the current time
    /// in the system in UTC.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.0
            .signed_duration_since(Utc::now())
            .abs()
            .to_std()
            .unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Eden timestamp")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timestamp::from_str(v).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.to_rfc3339(), f)
    }
}

impl FromStr for Timestamp {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DateTime::parse_from_rfc3339(s).map(|v| Self(v.to_utc()))
    }
}

impl From<NaiveDateTime> for Timestamp {
    fn from(value: NaiveDateTime) -> Self {
        Self(DateTime::<Utc>::from_naive_utc_and_offset(value, Utc))
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl From<Timestamp> for NaiveDateTime {
    fn from(value: Timestamp) -> Self {
        value.0.naive_utc()
    }
}

impl<'row> sqlx::Decode<'row, sqlx::Sqlite> for Timestamp
where
    DateTime<chrono::Utc>: sqlx::Decode<'row, sqlx::Sqlite>,
{
    fn decode(value: sqlx::sqlite::SqliteValueRef<'row>) -> Result<Self, sqlx::error::BoxDynError> {
        let dt = DateTime::<chrono::Utc>::decode(value)?;
        Ok(Self(dt))
    }
}

impl<'query> sqlx::Encode<'query, sqlx::Sqlite> for Timestamp
where
    DateTime<chrono::Utc>: sqlx::Encode<'query, sqlx::Sqlite>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'query>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        self.0.encode(buf)
    }
}

impl sqlx::Type<sqlx::Sqlite> for Timestamp
where
    String: sqlx::Type<sqlx::Sqlite>, //     DateTime<Utc>: sqlx::Type<sqlx::Sqlite>,
{
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[cfg(test)]
mod tests {
    use eden_sqlite::Pool;
    use sqlx::Row;

    use crate::timestamp::Timestamp;

    #[sqlx::test]
    async fn test_encode() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let now = Timestamp::now();

        let mut conn = pool.acquire().await.unwrap();
        let row = sqlx::query("SELECT ?")
            .bind(now)
            .fetch_one(&mut *conn)
            .await
            .unwrap();

        let result = row.try_get::<Timestamp, _>(0).unwrap();
        assert_eq!(now, result);

        let row = sqlx::query("SELECT ?")
            .bind(now)
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        let as_string = row.try_get::<String, _>(0).unwrap();
        assert!(as_string.ends_with("+00:00")); // make sure it ends with UTC zone
    }

    #[sqlx::test]
    async fn test_decode() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let row = sqlx::query("SELECT datetime(current_timestamp, 'utc')")
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        row.try_get::<Timestamp, _>(0).unwrap();
    }
}
