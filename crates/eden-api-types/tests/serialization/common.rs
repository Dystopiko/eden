use eden_api_types::Timestamp;
use eden_api_types::common::{ApiError, ApiErrorType, MemberDiscordInfo};
use std::str::FromStr;
use twilight_model::id::Id;

#[test]
fn error() {
    let payload = ApiError {
        error: ApiErrorType::Internal,
        // Since we have two possible types of message: either `Cow` or `String`
        // which it can be changed whether `server` feature is enabled.
        message: "Please try again later.".into(),
    };

    let json = serde_json::to_string_pretty(&payload).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn member_discord_info() {
    let payload = MemberDiscordInfo {
        id: Id::new(273534239310479360),
        name: "ferris".to_string(),
        joined_at: Timestamp::from_str("2024-01-01T00:00:00Z").unwrap(),
    };

    let json = serde_json::to_string_pretty(&payload).unwrap();
    insta::assert_snapshot!(json);
}
