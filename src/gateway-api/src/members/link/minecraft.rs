use eden_timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LinkMcAccount {
    pub uuid: Uuid,
    pub username: String,
    pub ip: IpAddr,
    pub java: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LinkChallenge {
    pub code: String,
    pub expires_at: Timestamp,
}
