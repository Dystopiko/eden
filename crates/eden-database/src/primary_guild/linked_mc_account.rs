use sqlx::{FromRow, Type};
use uuid::Uuid;

use crate::{Snowflake, Timestamp};

/// Differentiates between Minecraft editions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
#[sqlx(type_name = "mc_account_type", rename_all = "lowercase")]
pub enum McAccountType {
    Java,
    Bedrock,
}

/// A Minecraft account linked to a registered Discord user.
#[derive(Debug, Clone, FromRow)]
pub struct LinkedMcAccount {
    /// The Minecraft account UUID.
    ///
    /// - For Java accounts this is the Mojang-issued UUID.
    /// - For Bedrock accounts this is the XUID represented as a UUID.
    pub id: Uuid,

    /// The Discord user this account belongs to.
    pub discord_id: Snowflake,

    /// Whether this is a Java or Bedrock account
    pub account_type: McAccountType,

    /// The Minecraft username at the time of last sync.
    pub username: String,

    /// When this account was linked.
    pub linked_at: Timestamp,
}

impl LinkedMcAccount {
    /// Returns `true` if this account is a Java edition account.
    #[must_use]
    pub const fn is_java(&self) -> bool {
        matches!(self.account_type, McAccountType::Java)
    }

    /// Returns `true` if this account is a Bedrock edition account.
    #[must_use]
    pub const fn is_bedrock(&self) -> bool {
        matches!(self.account_type, McAccountType::Bedrock)
    }
}
