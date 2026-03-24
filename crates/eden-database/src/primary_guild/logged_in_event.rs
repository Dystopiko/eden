use bon::Builder;
use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::net::IpAddr;
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};
use uuid::Uuid;

use crate::{primary_guild::McAccountType, snowflake::Snowflake};

#[derive(Debug, Clone, FromRow)]
pub struct LoggedInEvent {
    pub event_id: Uuid,
    pub player_uuid: Uuid,
    pub created_at: Timestamp,
    pub username: String,
    #[sqlx(try_from = "crate::extractors::IpAddrString")]
    pub ip_address: IpAddr,
    #[sqlx(rename = "type")]
    pub kind: McAccountType,
    pub member_id: Option<Id<UserMarker>>,
}

impl LoggedInEvent {
    pub fn new_event() -> NewLoggedInEventBuilder {
        NewLoggedInEvent::builder()
    }
}

/// Error type representing a failure to query with the [`LoggedInEvent`] table.
#[derive(Debug, Error)]
#[error("Failed to query logged in event table from the database")]
pub struct LoggedInEventQueryError;

#[derive(Builder, Debug, Deserialize, Serialize)]
pub struct NewLoggedInEvent {
    #[builder(default = Uuid::new_v4())]
    pub event_id: Uuid,
    pub player_uuid: Uuid,
    #[builder(default = Timestamp::now())]
    pub created_at: Timestamp,
    pub username: Option<String>,
    pub ip_address: IpAddr,
    pub kind: McAccountType,
    pub member_id: Option<Id<UserMarker>>,
}

impl NewLoggedInEvent {
    pub async fn create(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<(), Report<LoggedInEventQueryError>> {
        sqlx::query(
            r#"
            INSERT INTO logged_in_events (
                event_id, player_uuid, created_at, username,
                ip_address, "type", member_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(self.event_id)
        .bind(self.player_uuid)
        .bind(self.created_at)
        .bind(&self.username)
        .bind(self.ip_address.to_string())
        .bind(self.kind)
        .bind(self.member_id.map(|v| Snowflake::new(v.cast())))
        .execute(&mut **conn)
        .await
        .change_context(LoggedInEventQueryError)
        .attach("while trying to log login event")?;

        Ok(())
    }
}
