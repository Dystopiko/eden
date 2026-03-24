use eden_timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(validator::Validate))]
pub struct LinkMcAccount {
    pub uuid: Uuid,
    #[cfg_attr(
        feature = "server",
        validate(custom(
            function = "eden_validation::minecraft::validate_username",
            message = "Invalid Minecraft username"
        ))
    )]
    pub username: String,
    pub ip: IpAddr,
    pub java: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LinkChallenge {
    pub code: String,
    pub expires_at: Timestamp,
}
