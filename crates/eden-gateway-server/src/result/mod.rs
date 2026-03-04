use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

// `ErasedReport` is intentionally kept within this crate instead of the
// eden-common crate to ensure all errors are always associated with a
// concrete context type at their origin in other crates.
mod erased_report;
pub use self::erased_report::{EraseReportExt, ErasedReport};

#[cfg(test)]
mod tests;

pub type ApiResult<T> = std::result::Result<T, ErasedReport>;

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: Cow<'static, str>,

    /// A unique ID to correlate with the server logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
}

impl ApiError {
    pub const INTERNAL: Self = Self::from_static(
        ApiErrorCode::Internal,
        "An unexpected error occurred while handling your request. \
        Please try again later, or contact a server administrator if the issue persists.",
    );

    pub const NOT_FOUND: Self = Self::from_static(
        ApiErrorCode::NotFound,
        "The requested resource could not be found.",
    );

    pub const READONLY_MODE: Self = Self::from_static(
        ApiErrorCode::ReadonlyMode,
        "Eden is temporarily operating in read-only mode. \
        Check the announcements for updates from a server administrator and try again later.",
    );

    pub const SERVICE_UNAVAILABLE: Self = Self::from_static(
        ApiErrorCode::ServiceUnavailable,
        "Eden is temporarily unavailable. \
        Check the announcements for updates from a server administrator and try again later.",
    );

    #[must_use]
    const fn from_static(code: ApiErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: Cow::Borrowed(message),
            request_id: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }
}

// impl<E: StdError> From<Report<E>> for Error {
//     fn from(report: Report<E>) -> Self {
//         // if let Some(error) = report.downcast_ref::<PoolError>() {
//         //     match error {
//         //         PoolError::General => {
//         //             tracing::error!(error = ?report, "encountered a pool error");
//         //             return Error::INTERNAL;
//         //         }
//         //         PoolError::Unhealthy => return Error::SERVICE_UNAVAILABLE,
//         //     };
//         // }

//         // if let Some(kind) = report.sql_error_type() {
//         //     match kind {
//         //         SqlErrorType::Readonly => return Error::READONLY_MODE,
//         //         SqlErrorType::UnhealthyConnection => return Error::SERVICE_UNAVAILABLE,
//         //         SqlErrorType::Unknown => {
//         //             tracing::error!(error = ?report, "encountered a database error");
//         //             return Error::INTERNAL;
//         //         }
//         //         _ => {}
//         //     };
//         // }

//         // tracing::error!(error = ?report, "unhandled error while processing request");
//         Error::INTERNAL
//     }
// }

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status: StatusCode = self.code.into();
        (status, Json(self)).into_response()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorCode {
    Internal,
    ReadonlyMode,
    NotFound,
    InvalidRequest,
    ServiceUnavailable,
}

impl From<ApiErrorCode> for StatusCode {
    fn from(code: ApiErrorCode) -> Self {
        match code {
            ApiErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::ReadonlyMode | ApiErrorCode::ServiceUnavailable => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
            ApiErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        }
    }
}
