use eden_database::{
    primary_guild::{McAccount, Member},
    views::{McAccountView, MemberView},
};

use crate::members::{EncodedMember, FullMember, MinimalMcAccount};

impl From<(MemberView, Vec<McAccount>)> for FullMember {
    fn from((view, accounts): (MemberView, Vec<McAccount>)) -> Self {
        Self {
            id: view.discord_user_id.cast(),
            name: view.name,
            rank: view.rank.to_string(),
            accounts: accounts.into_iter().map(Into::into).collect(),
            invited_by: view
                .invited_by
                .zip(view.inviter_name)
                .map(|(id, name)| EncodedMember {
                    id: id.cast(),
                    name,
                    rank: None,
                }),

            // TODO: fill up the missing fields
            last_login_at: None,
            last_account: None,
        }
    }
}

impl From<(MemberView, Vec<MinimalMcAccount>)> for FullMember {
    fn from((view, accounts): (MemberView, Vec<MinimalMcAccount>)) -> Self {
        Self {
            id: view.discord_user_id.cast(),
            name: view.name,
            rank: view.rank.to_string(),
            accounts,
            invited_by: view
                .invited_by
                .zip(view.inviter_name)
                .map(|(id, name)| EncodedMember {
                    id: id.cast(),
                    name,
                    rank: None,
                }),

            // TODO: fill up the missing fields
            last_login_at: None,
            last_account: None,
        }
    }
}

impl From<Member> for EncodedMember {
    fn from(value: Member) -> Self {
        Self {
            id: value.discord_user_id.cast(),
            name: value.name,
            rank: None,
        }
    }
}

impl From<MemberView> for EncodedMember {
    fn from(member: MemberView) -> Self {
        Self {
            id: member.discord_user_id.cast(),
            name: member.name,
            rank: Some(member.rank.to_string()),
        }
    }
}

impl From<McAccountView> for EncodedMember {
    fn from(view: McAccountView) -> Self {
        Self {
            id: view.member_id.cast(),
            name: view.member_name,
            rank: Some(view.member_rank.to_string()),
        }
    }
}
