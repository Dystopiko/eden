use serde::{Deserialize, Serialize};
use twilight_model::id::{Id, marker::UserMarker};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncodedMember {
    pub id: Id<UserMarker>,
    pub name: String,
}
