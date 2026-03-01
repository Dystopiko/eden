use eden_database::Timestamp;
use eden_database::primary_guild::member::DbMember;

use serde::Serialize;
use twilight_model::id::{Id, marker::GuildMarker};

#[derive(Debug, Clone, Serialize)]
pub struct MemberView {
    pub id: Id<GuildMarker>,
    pub name: String,
    pub joined_at: Timestamp,
}

impl MemberView {
    #[must_use]
    pub fn from_full(member: DbMember) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
            joined_at: member.joined_at,
        }
    }
}
