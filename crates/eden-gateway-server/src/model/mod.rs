use eden_database::primary_guild::Member;
use serde::{Deserialize, Serialize};
use twilight_model::id::{Id, marker::UserMarker};

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
