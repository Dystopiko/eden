use crate::Timestamp;
use chrono::DateTime;

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
    String: sqlx::Type<sqlx::Sqlite>,
{
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[cfg(test)]
mod tests {
    use eden_sqlite::Pool;
    use sqlx::Row;

    use crate::Timestamp;

    #[sqlx::test]
    async fn test_encode() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
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
        let row = sqlx::query("SELECT datetime(current_timestamp, 'utc')")
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        row.try_get::<Timestamp, _>(0).unwrap();
    }
}
