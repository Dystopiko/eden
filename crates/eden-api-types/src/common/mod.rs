use eden_timestamp_type::Timestamp;
use serde::{Deserialize, Serialize};
use twilight_model::id::{Id, marker::GuildMarker};

#[cfg(feature = "server")]
use std::borrow::Cow;

// For now, we're not going to overcomplicate ourselves of what
// we want for an API error to be.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub error: ApiErrorType,
    #[cfg(feature = "server")]
    pub message: Cow<'static, str>,
    #[cfg(not(feature = "server"))]
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorType {
    Internal,
    #[serde(rename = "invalid_request")]
    Request,
    NotFound,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MemberDiscordInfo {
    pub id: Id<GuildMarker>,
    pub name: String,
    pub joined_at: Timestamp,
}
