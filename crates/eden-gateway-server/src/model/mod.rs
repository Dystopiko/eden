use eden_database::primary_guild::Member;
use serde::{Deserialize, Serialize};
use std::fmt;
use twilight_model::id::{Id, marker::UserMarker};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum McGameMode {
    #[serde(rename = "minecraft:survival")]
    Survival,

    #[serde(rename = "minecraft:creative")]
    Creative,

    #[serde(rename = "minecraft:adventure")]
    Adventure,

    #[serde(rename = "minecraft:spectator")]
    Spectator,
}

impl fmt::Display for McGameMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Survival => f.write_str("minecraft:survival"),
            Self::Creative => f.write_str("minecraft:creative"),
            Self::Adventure => f.write_str("minecraft:adventure"),
            Self::Spectator => f.write_str("minecraft:spectator"),
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct McBlockPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemberView {
    pub id: Id<UserMarker>,
    pub name: String,
}

impl From<Member> for MemberView {
    fn from(member: Member) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
        }
    }
}
