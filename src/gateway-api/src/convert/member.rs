use eden_database::views::{McAccountView, MemberView};

use crate::members::EncodedMember;

impl From<MemberView> for EncodedMember {
    fn from(member: MemberView) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
            rank: member.rank.to_string(),
        }
    }
}

impl From<McAccountView> for EncodedMember {
    fn from(view: McAccountView) -> Self {
        Self {
            id: view.member_id.cast(),
            name: view.member_name,
            rank: view.member_rank.to_string(),
        }
    }
}
