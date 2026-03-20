use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;

use crate::Timestamp;

/// Provides a behavioral metrics record for Chaos (chaosneco).
#[derive(Debug, FromRow)]
pub struct Chaos {
    pub id: i32,
    pub created_at: Timestamp,
    pub crying_emoticon_times: i32,
    pub updated_at: Timestamp,
}

/// Error type representing a failure to interact with the Chaos metrics table.
#[derive(Debug, Error)]
#[error("Could not update Chaos metrics table entry in the database")]
pub struct UpdateChaosError;

impl Chaos {
    pub async fn add_crying_times(
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<Self, Report<UpdateChaosError>> {
        sqlx::query_as::<_, Chaos>(
            r#"
        INSERT INTO chaos_metrics (id, crying_emoticon_times)
        VALUES (1, 1)
        ON CONFLICT (id) DO UPDATE
            SET crying_emoticon_times = (chaos_metrics.crying_emoticon_times + 1) % 2147483647,
                updated_at = datetime(current_timestamp, 'utc')
        RETURNING *
        "#,
        )
        .fetch_one(&mut **conn)
        .await
        .change_context(UpdateChaosError)
    }
}

#[cfg(test)]
mod tests {
    use crate::primary_guild::Chaos;

    #[tokio::test]
    async fn test_overflow() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        Chaos::add_crying_times(&mut conn).await.unwrap();

        sqlx::query(
            r"
        UPDATE chaos_metrics
        SET crying_emoticon_times = 2147483647
        WHERE id = 1",
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        let info = Chaos::add_crying_times(&mut conn).await.unwrap();
        assert_eq!(info.crying_emoticon_times, 1, "should revert back to 1");
    }

    #[tokio::test]
    async fn should_increment_first_crying_times() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let info = Chaos::add_crying_times(&mut conn).await.unwrap();

        assert_eq!(info.id, 1);
        assert_eq!(info.crying_emoticon_times, 1);
    }

    #[tokio::test]
    async fn should_increment_existing_crying_times() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let initial = Chaos::add_crying_times(&mut conn).await.unwrap();
        let info = Chaos::add_crying_times(&mut conn).await.unwrap();

        assert_eq!(info.id, 1);
        assert_eq!(info.crying_emoticon_times, 2);
        assert_eq!(info.created_at, initial.created_at);
        assert_ne!(info.updated_at, initial.updated_at);
    }
}
