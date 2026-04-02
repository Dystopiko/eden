use eden_database::{Timestamp, primary_guild::McAccountType};
use serde::{Deserialize, Serialize};
use twilight_model::id::{Id, marker::UserMarker};
use uuid::Uuid;

pub mod link;

/// Full metadata of a member that only can be accessed by an administrator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FullMember {
    pub id: Id<UserMarker>,
    pub name: String,
    pub rank: String,
    pub invited_by: Option<EncodedMember>,
    pub last_login_at: Option<Timestamp>,
    pub last_account: Option<Uuid>,
    pub accounts: Vec<MinimalMcAccount>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinimalMcAccount {
    pub linked_at: Timestamp,
    pub uuid: Uuid,
    pub username: String,
    #[serde(rename = "type")]
    pub kind: McAccountType,
}

/// Minimal metadata of a member. It is used in many routes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncodedMember {
    pub id: Id<UserMarker>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
}
