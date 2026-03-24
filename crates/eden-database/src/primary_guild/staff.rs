use bon::Builder;
use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};

use crate::Snowflake;

#[derive(Clone, Debug, FromRow)]
pub struct Staff {
    pub member_id: Snowflake,
    pub joined_at: Timestamp,
    pub updated_at: Option<Timestamp>,
    pub admin: bool,
}

impl Staff {
    pub async fn delete(
        conn: &mut eden_sqlite::Connection,
        id: Id<UserMarker>,
    ) -> Result<(), Report<StaffQueryError>> {
        sqlx::query(
            r#"
            DELETE FROM staffs
            WHERE member_id = = ?"#,
        )
        .bind(Snowflake::new(id.cast()))
        .execute(conn)
        .await
        .change_context(StaffQueryError)
        .attach("while trying to delete staff by member id")?;

        Ok(())
    }

    pub async fn find_by_member_id(
        conn: &mut eden_sqlite::Connection,
        id: Id<UserMarker>,
    ) -> Result<Self, Report<StaffQueryError>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM staffs
            WHERE member_id = = ?"#,
        )
        .bind(Snowflake::new(id.cast()))
        .fetch_one(conn)
        .await
        .change_context(StaffQueryError)
        .attach("while trying to find staff row by member id")
    }

    pub fn upsert() -> UpsertStaffBuilder {
        UpsertStaff::builder()
    }
}

/// Error type representing a failure to query with the [`Staff`] table.
#[derive(Debug, Error)]
#[error("Failed to query staffs table from the database")]
pub struct StaffQueryError;

#[derive(Builder)]
pub struct UpsertStaff {
    pub member_id: Snowflake,
    #[builder(default = Timestamp::now())]
    pub joined_at: Timestamp,
    #[builder(default = false)]
    pub admin: bool,
}

impl UpsertStaff {
    pub async fn perform(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<(), Report<StaffQueryError>> {
        sqlx::query(
            r#"
            INSERT INTO staffs (member_id, joined_at, admin)
            VALUES (?, ?, ?)
            ON CONFLICT (member_id)
                DO UPDATE
                SET admin = excluded.admin,
                    updated_at = current_timestamp
            RETURNING *"#,
        )
        .bind(Snowflake::new(self.member_id.cast()))
        .bind(self.joined_at)
        .bind(self.admin)
        .execute(&mut **conn)
        .await
        .change_context(StaffQueryError)
        .attach("while trying to upsert staff")?;

        Ok(())
    }
}
