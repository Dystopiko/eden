use bon::Builder;
use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};

use crate::Snowflake;

/// Represents a member who joined the primary guild's Minecraft server.
#[derive(Debug, Clone, FromRow)]
pub struct Contributor {
    pub member_id: Snowflake,
    pub created_at: Timestamp,
    pub updated_at: Option<Timestamp>,
}

impl Contributor {
    pub fn upsert() -> UpsertContributorBuilder {
        UpsertContributor::builder()
    }
}

/// Error type representing a failure to query with the [`Contributor`] table.
#[derive(Debug, Error)]
#[error("Failed to query contributors table from the database")]
pub struct ContributorQueryError;

#[derive(Builder)]
pub struct UpsertContributor {
    pub member_id: Id<UserMarker>,
    #[builder(default = Timestamp::now())]
    pub created_at: Timestamp,
}

impl UpsertContributor {
    pub async fn perform(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<(), Report<ContributorQueryError>> {
        sqlx::query(
            r#"
            INSERT INTO contributors(member_id, created_at)
            VALUES (?, ?)"#,
        )
        .bind(Snowflake::new(self.member_id.cast()))
        .bind(self.created_at)
        .execute(&mut **conn)
        .await
        .change_context(ContributorQueryError)
        .attach("while trying to upsert contributor")?;

        Ok(())
    }
}
