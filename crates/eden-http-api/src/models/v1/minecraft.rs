use eden_database::Timestamp;
use eden_database::primary_guild::minecraft::{DbMinecraftAccount, McAccountType};

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct MinecraftAccount {
    #[serde(rename = "type")]
    pub kind: McAccountType,
    pub linked_at: Timestamp,
    pub uuid: Uuid,
    pub username: String,
}

impl MinecraftAccount {
    #[must_use]
    pub fn from_full(account: DbMinecraftAccount) -> MinecraftAccount {
        MinecraftAccount {
            kind: account.kind,
            linked_at: account.linked_at,
            uuid: account.uuid,
            username: account.username,
        }
    }
}
