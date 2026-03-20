use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::member::EncodedMember;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(validator::Validate))]
pub struct ValidatePlayers {
    #[cfg_attr(
        feature = "server",
        validate(
            length(min = 1, max = 100),
            custom(
                function = "eden_validation::no_duplicated_entry",
                message = "Every provided player UUIDs must be unique"
            ),
        )
    )]
    pub players: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidatePlayersResponse {
    pub players: HashMap<Uuid, Option<PlayerEntry>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlayerEntry {
    pub member: EncodedMember,
    pub perks: Vec<String>,
}
