use serde::{Deserialize, Serialize};
use twilight_model::id::{Id, marker::UserMarker};

use crate::members::EncodedMember;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PatchMember {
    pub name: Option<String>,
    pub invited_by: Option<Id<UserMarker>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Invitees {
    pub count: u64,
    pub invitees: Vec<EncodedMember>,
}
