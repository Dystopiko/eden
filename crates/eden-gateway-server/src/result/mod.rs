use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
mod tests;

pub type ApiResult<T> = std::result::Result<T, ApiErrorOutcome>;

/// A serializable error response returned to API clients.
///
/// Contains a machine-readable [`ErrorCode`], a human-readable message, and an optional
/// [`Uuid`] that correlates the response with server-side logs. Several common errors are
/// provided as associated constants for convenience.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: Cow<'static, str>,

    /// A unique ID to correlate with the server logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
}

impl ApiError {
    pub const INTERNAL: Self = Self::from_static(
        ErrorCode::Internal,
        "An unexpected error occurred while handling your request. \
        Please try again later, or contact a server administrator if the issue persists.",
    );

    pub const NOT_FOUND: Self = Self::from_static(
        ErrorCode::NotFound,
        "The requested resource could not be found.",
    );

    pub const READONLY_MODE: Self = Self::from_static(
        ErrorCode::ReadonlyMode,
        "Eden is temporarily operating in read-only mode. \
        Check the announcements for updates from a server administrator and try again later.",
    );

    pub const SERVICE_UNAVAILABLE: Self = Self::from_static(
        ErrorCode::ServiceUnavailable,
        "Eden is temporarily unavailable. \
        Check the announcements for updates from a server administrator and try again later.",
    );

    /// Creates a new [`ApiError`] with the given [`ErrorCode`] and static message.
    #[must_use]
    pub(crate) const fn from_static(code: ErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: Cow::Borrowed(message),
            request_id: None,
        }
    }

    /// Attaches a request ID to correlate this error with server-side logs.
    #[must_use]
    pub fn with_request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Attaches a maybe [`Some`] request ID to correlate this error with server-side logs.
    #[must_use]
    pub fn maybe_request_id(mut self, id: Option<Uuid>) -> Self {
        self.request_id = id;
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status: StatusCode = self.code.into();
        (status, Json(self)).into_response()
    }
}

/// A machine-readable classification of an API error.
///
/// Serialized as `SCREAMING_SNAKE_CASE` in JSON responses (e.g. `"INVALID_REQUEST"`),
/// and mapped to an appropriate HTTP status code via [`From<ErrorCode> for StatusCode`].
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Maps to `500 Internal Server Error`.
    Internal,

    /// Maps to `503 Service Unavailable`.
    ReadonlyMode,

    /// Maps to `404 Not Found`.
    NotFound,

    /// Maps to `400 Bad Request`.
    InvalidRequest,

    /// Maps to `503 Service Unavailable`.
    ServiceUnavailable,
}

impl From<ErrorCode> for StatusCode {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ReadonlyMode | ErrorCode::ServiceUnavailable => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        }
    }
}

/// The error type for [`ApiResult`], carrying either a structured [`ApiError`] or an
/// unhandled [`ErasedReport`].
///
/// Structured errors are serialized directly into the response body. Reports are
/// attached to the response extensions for server-side logging and surfaced to the
/// client as [`ApiError::INTERNAL`] to avoid leaking internal details.
pub enum ApiErrorOutcome {
    /// A structured error with a known [`ErrorCode`] and message.
    Unhandled(ErasedReport),

    /// An opaque, unexpected failure. Logged server-side; client receives
    /// [`ApiError::INTERNAL`].
    Known(ApiError),
}

impl<R> From<R> for ApiErrorOutcome
where
    R: Into<ErasedReport>,
{
    fn from(value: R) -> Self {
        Self::Unhandled(value.into())
    }
}

impl From<ApiError> for ApiErrorOutcome {
    fn from(value: ApiError) -> Self {
        Self::Known(value)
    }
}

impl IntoResponse for ApiErrorOutcome {
    fn into_response(self) -> Response {
        match self {
            Self::Known(error) => error.into_response(),
            Self::Unhandled(report) => {
                let mut res = ApiError::INTERNAL.into_response();
                res.extensions_mut().insert(Arc::new(report));
                res
            }
        }
    }
}
