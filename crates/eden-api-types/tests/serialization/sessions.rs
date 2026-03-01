use eden_api_types::Timestamp;
use eden_api_types::common::MemberDiscordInfo;
use eden_api_types::sessions::{RequestSession, RequestSessionResponse};
use std::net::IpAddr;
use std::str::FromStr;
use twilight_model::id::Id;
use uuid::Uuid;

#[test]
fn rejected_request_session_response() {
    let response = RequestSessionResponse::Rejected {
        reason: "This server is currently in locked down mode".to_string(),
        note: Some("Please come to us again later once it is free for everyone".to_string()),
    };

    let json = serde_json::to_string_pretty(&response).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn granted_request_session_response() {
    let response = RequestSessionResponse::Granted {
        last_login_at: Some(Timestamp::from_str("2024-01-01T00:00:00Z").unwrap()),
        perks: vec![
            "keep-inventory".into(),
            "instant-restock".into(),
            "veinminer".into(),
        ],
        discord: Some(MemberDiscordInfo {
            id: Id::new(123456789012345678),
            joined_at: Timestamp::from_str("2024-01-01T00:00:00Z").unwrap(),
            name: "john".to_string(),
        }),
    };

    let json = serde_json::to_string_pretty(&response).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn request_session() {
    let uuid = "567458bd-d97f-4fe1-8123-6b380998acbe";
    let ip = "192.168.0.1";

    let session = RequestSession {
        uuid: Uuid::from_str(uuid).unwrap(),
        ip: IpAddr::from_str(ip).unwrap(),
        bedrock: false,
    };

    let json = serde_json::to_string_pretty(&session).unwrap();
    insta::assert_snapshot!(json);
}
