use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;

use crate::Timestamp;

/// Represents a record in the dedicated Chaos table.
///
/// This struct stores persistent information about "Chaos"
/// within the configured primary guild, specifically tracking
/// behavioral metrics.
#[derive(Debug, FromRow)]
pub struct Chaos {
    pub id: i32,
    pub crying_times: i64,
    pub updated_at: Timestamp,
}

/// Error type representing a failure to interact with the Chaos table.
#[derive(Debug, Error)]
#[error("Could not update Chaos table entry in the database")]
pub struct UpdateChaosError;

impl Chaos {
    #[tracing::instrument(skip_all, name = "db.chaos.increment_crying_times")]
    pub async fn increment_crying_times(
        conn: &mut eden_sqlite::Connection,
    ) -> Result<Self, Report<UpdateChaosError>> {
        sqlx::query_as::<_, Chaos>(
            r#"
        INSERT INTO "primary_guild.chaos" (id, crying_times, updated_at)
        VALUES (1, 1, ?)
        ON CONFLICT (id) DO UPDATE
            SET crying_times = "primary_guild.chaos".crying_times + 1,
                updated_at = datetime(current_timestamp, 'utc')
        RETURNING *
        "#,
        )
        .bind(Timestamp::now())
        .fetch_one(conn)
        .await
        .change_context(UpdateChaosError)
    }
}

#[cfg(test)]
mod tests {
    use eden_sqlite::Pool;

    use crate::primary_guild::Chaos;

    #[tokio::test]
    async fn should_increment_first_crying_times() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let info = Chaos::increment_crying_times(&mut conn).await.unwrap();

        assert_eq!(info.id, 1);
        assert_eq!(info.crying_times, 1);
    }

    #[tokio::test]
    async fn should_increment_existing_crying_times() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        Chaos::increment_crying_times(&mut conn).await.unwrap();

        let info = Chaos::increment_crying_times(&mut conn).await.unwrap();
        assert_eq!(info.id, 1);
        assert_eq!(info.crying_times, 2);
    }
}
