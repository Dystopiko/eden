use eden_timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use crate::members::EncodedMember;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestSession {
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub java: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SessionGranted {
    pub last_login_at: Option<Timestamp>,
    pub member: Option<EncodedMember>,
    pub perks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_serialization_of_request_session() {
        let alert = RequestSession {
            uuid: Uuid::nil(),
            ip: IpAddr::from_str("127.0.0.1").unwrap(),
            java: true,
        };
        insta::assert_json_snapshot!(alert);
    }

    #[test]
    fn test_serialization_of_session_granted() {
        let alert = SessionGranted {
            last_login_at: Some(Timestamp::from_secs(1234567).unwrap()),
            member: None,
            perks: vec!["stocks".to_string()],
        };
        insta::assert_json_snapshot!(alert);
    }
}
