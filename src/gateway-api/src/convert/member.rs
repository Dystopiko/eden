use eden_database::primary_guild::Member;

use crate::member::EncodedMember;

impl EncodedMember {
    #[must_use]
    pub fn from_db(member: Member) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
        }
    }
}
