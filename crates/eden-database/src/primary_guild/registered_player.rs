use sqlx::FromRow;

use crate::{Snowflake, Timestamp};

/// Represents a Discord user registered to the primary guild's Minecraft server.
#[derive(Debug, Clone, FromRow)]
pub struct RegisteredPlayer {
    /// The player's Discord snowflake ID.
    pub discord_user_id: Snowflake,

    /// When this registration was created.
    pub created_at: Timestamp,

    /// The player's Discord username.
    pub name: String,

    /// When this row was updated.
    pub updated_at: Option<Timestamp>,
}
