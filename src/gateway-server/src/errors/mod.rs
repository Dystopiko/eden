use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use eden_sqlite::error::{PoolError, ReportExt, SqlErrorType};
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};
use uuid::Uuid;
use validator::ValidationErrors;

use crate::ratelimiter::TooManyRequests;

/// A serializable error response returned to API clients.
///
/// It contains a machine-readable [`ErrorCode`], a human-readable message, and an optional
/// [`Uuid`] that correlates the response with server-side logs. Several common errors are
/// provided as associated constants for convenience.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: Cow<'static, str>,

    /// Additional headers to be embedded in the associated HTTP response.
    #[serde(skip)]
    pub headers: Option<HeaderMap>,

    /// The original unhandled report, kept out of serialization and only
    /// used when converting this error into a [`Response`].
    #[serde(skip)]
    pub report: Option<ErasedReport>,

    /// A unique ID to correlate with the server logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
}

/// A machine-readable classification of an API error.
///
/// Serialized as `SCREAMING_SNAKE_CASE` in JSON responses (e.g. `"NOT_FOUND"`),
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
    /// Maps to `429 Too Many Requests`
    RateLimited,
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
    pub const fn from_static(code: ErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: Cow::Borrowed(message),
            headers: None,
            report: None,
            request_id: None,
        }
    }

    /// Creates a new [`ApiError`] with the given [`ErrorCode`] and owned message.
    #[must_use]
    pub fn from_owned(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: Cow::Owned(message.into()),
            headers: None,
            report: None,
            request_id: None,
        }
    }

    /// Attaches a request ID to correlate this error with server-side logs.
    #[must_use]
    pub fn with_request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Attaches an optional request ID, clearing it if [`None`] is passed.
    #[must_use]
    pub fn maybe_request_id(mut self, id: Option<Uuid>) -> Self {
        self.request_id = id;
        self
    }

    /// Attaches an unhandled report to this error.
    ///
    /// The report is stashed in the response extensions by [`IntoResponse`] so
    /// that `normalize_error` can log it with full span context. It is never
    /// serialized to the client.
    #[must_use]
    pub(crate) fn with_report(mut self, report: ErasedReport) -> Self {
        self.report = Some(report);
        self
    }
}

impl ApiError {
    #[must_use]
    pub(crate) fn from_validate(error: ValidationErrors) -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: Cow::Owned(error.to_string()),
            headers: None,
            report: None,
            request_id: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(mut self) -> Response {
        let status: StatusCode = self.code.into();
        let report = self.report.take().map(Arc::new);

        // Serialize without the report field (it is `#[serde(skip)]`).
        let mut response = (status, Json(&self)).into_response();
        if let Some(report) = report {
            response.extensions_mut().insert(report);
        }

        // Include every headers provided by the error
        if let Some(headers) = self.headers.take() {
            response.headers_mut().extend(headers);
        }

        response
    }
}

impl<R> From<R> for ApiError
where
    R: Into<ErasedReport>,
{
    fn from(report: R) -> Self {
        ApiError::INTERNAL.with_report(report.into())
    }
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
            ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

/// Classifies an [`ErasedReport`] into the most appropriate [`ApiError`].
///
/// Types that want to influence error classification should implement
/// [`HttpErrorClass`] and register themselves here. Unrecognized errors are
/// logged at `error` level and returned as [`ApiError::INTERNAL`].
pub fn classify(report: ErasedReport) -> ApiError {
    // Each registered classifier is tried in order. The first match wins.
    let classifiers: &[fn(&ErasedReport) -> Option<ApiError>] =
        &[classify_db, classify_rate_limit_error];

    for classify in classifiers {
        if let Some(error) = classify(&report) {
            return error;
        }
    }

    tracing::error!(error = ?report, "unhandled error while processing request");
    ApiError::INTERNAL
}

fn classify_rate_limit_error(report: &ErasedReport) -> Option<ApiError> {
    let error = report.downcast_ref::<TooManyRequests>()?;
    Some(ApiError {
        code: ErrorCode::RateLimited,
        message: Cow::Borrowed(error.action().message()),
        headers: Some(error.stats().into_headers()),
        report: None,
        request_id: None,
    })
}

fn classify_db(report: &ErasedReport) -> Option<ApiError> {
    if let Some(kind) = report.sql_error_type() {
        return Some(match kind {
            SqlErrorType::Readonly => ApiError::READONLY_MODE,
            SqlErrorType::UnhealthyConnection => ApiError::SERVICE_UNAVAILABLE,
            SqlErrorType::Unknown => {
                tracing::error!(error = ?report, "encountered a database error");
                ApiError::INTERNAL
            }
            _ => return None,
        });
    }

    if let Some(pool_error) = report.downcast_ref::<PoolError>() {
        return Some(match pool_error {
            PoolError::General => {
                tracing::error!(error = ?report, "encountered a pool error");
                ApiError::INTERNAL
            }
            PoolError::Unhealthy => ApiError::SERVICE_UNAVAILABLE,
        });
    }

    None
}

#[cfg(test)]
mod tests;
