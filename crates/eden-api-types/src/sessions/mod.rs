use eden_timestamp_type::Timestamp;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use crate::common::MemberDiscordInfo;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RequestSession {
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub bedrock: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum RequestSessionResponse {
    Granted {
        last_login_at: Option<Timestamp>,
        perks: Vec<String>,
        discord: Option<MemberDiscordInfo>,
    },
    Rejected {
        reason: String,
        note: Option<String>,
    },
}
