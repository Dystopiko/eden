use eden_database::primary_guild::McAccount;

use crate::members::MinimalMcAccount;

impl From<McAccount> for MinimalMcAccount {
    fn from(value: McAccount) -> Self {
        Self {
            linked_at: value.linked_at,
            uuid: value.uuid,
            username: value.username,
            kind: value.kind,
        }
    }
}
