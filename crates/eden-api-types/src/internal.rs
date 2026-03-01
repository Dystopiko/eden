use eden_database::primary_guild::Member;

use crate::common::MemberDiscordInfo;

impl MemberDiscordInfo {
    #[must_use]
    pub fn from_db(member: Member) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
            joined_at: member.joined_at,
        }
    }
}
