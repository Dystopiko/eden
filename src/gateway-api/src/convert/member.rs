use eden_database::{primary_guild::Member, views::McAccountView};

use crate::members::EncodedMember;

impl From<Member> for EncodedMember {
    fn from(member: Member) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
        }
    }
}

impl From<McAccountView> for EncodedMember {
    fn from(view: McAccountView) -> Self {
        Self {
            id: view.member_id.cast(),
            name: view.member_name,
        }
    }
}
