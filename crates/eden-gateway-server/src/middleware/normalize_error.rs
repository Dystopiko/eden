use eden_sqlite::error::{PoolError, ReportExt, SqlErrorType};
use erased_report::ErasedReport;
use error_stack::{Report, ResultExt};
use mime::Mime;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use axum::{
    extract::{Json, Request},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    middleware::trace_request::RequestId,
    result::{ApiError, ErrorCode},
};

#[derive(Debug, Error)]
#[error("failed to convert plain-text error body to JSON")]
struct JsonifyError;

/// This that normalizes all error responses into a consistent [`ApiError`]
/// payload in JSON format.
///
/// The middleware only activates for responses whose HTTP status indicates a
/// client error (4xx) or server error (5xx); successful responses are passed
/// through untouched.
pub async fn middleware(req: Request, next: Next) -> impl IntoResponse {
    let mut res = next.run(req).await;

    let status = res.status();
    if !status.is_client_error() && !status.is_server_error() {
        return res;
    }

    // Try to get the associated request ID to be easily traceable soon.
    let request_id = res.extensions().get::<RequestId>().map(|v| v.0);

    // Try to get the erased report data via extensions
    if let Some(report) = take_erased_report(&mut res) {
        return report_to_api_error(report)
            .maybe_request_id(request_id)
            .into_response();
    }

    match jsonify_plain_text_error(res, request_id).await {
        Ok(converted) => converted,
        Err(err) => {
            tracing::error!(error = ?err, "failed to jsonify plain-text error response");
            ApiError::INTERNAL
                .maybe_request_id(request_id)
                .into_response()
        }
    }
}

/// Converts a `text/plain; charset=utf-8` error body into a JSON `ApiError`.
/// Responses with any other content type are returned unchanged.
async fn jsonify_plain_text_error(
    res: Response,
    request_id: Option<Uuid>,
) -> Result<Response, Report<JsonifyError>> {
    const MAX_BODY_BYTES: usize = 1_000_000;

    let (mut parts, body) = res.into_parts();
    if !is_plain_text_utf8(&parts.headers) {
        return Ok((parts, body).into_response());
    }

    let bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
        .await
        .change_context(JsonifyError)
        .attach("while trying to read the response body")?;

    let message = String::from_utf8(bytes.into())
        .change_context(JsonifyError)
        .attach("while trying to decoding the response body as UTF-8")?;

    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);

    let payload = ApiError {
        code: ErrorCode::InvalidRequest,
        message: Cow::Owned(message),
        request_id,
    };

    Ok((parts, Json(payload)).into_response())
}

fn take_erased_report(res: &mut Response) -> Option<ErasedReport> {
    res.extensions_mut()
        .remove::<Arc<ErasedReport>>()
        .and_then(Arc::into_inner)
}

fn is_plain_text_utf8(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Mime::from_str(v).ok())
        .is_some_and(|mime| mime == mime::TEXT_PLAIN_UTF_8)
}

/// Converts a raw `ErasedReport` into the most appropriate `ApiError` variant,
/// logging unexpected errors at the `error` level.
fn report_to_api_error(report: ErasedReport) -> ApiError {
    if let Some(inner) = report.downcast_ref::<PoolError>() {
        return match inner {
            PoolError::General => {
                tracing::error!(error = ?report, "encountered a pool error");
                ApiError::INTERNAL
            }
            PoolError::Unhealthy => ApiError::SERVICE_UNAVAILABLE,
        };
    }

    if let Some(kind) = report.sql_error_type() {
        match kind {
            SqlErrorType::Readonly => return ApiError::READONLY_MODE,
            SqlErrorType::UnhealthyConnection => return ApiError::SERVICE_UNAVAILABLE,
            SqlErrorType::Unknown => {
                tracing::error!(error = ?report, "encountered a database error");
                return ApiError::INTERNAL;
            }
            _ => {}
        };
    }

    tracing::error!(error = ?report, "unhandled error while processing request");
    ApiError::INTERNAL
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, Bytes},
        extract::Json,
        http::{Method, Request, StatusCode, response::Parts},
        routing::{Router, get},
    };

    use error_stack::Report;
    use pretty_assertions::assert_eq;
    use std::borrow::Cow;
    use thiserror::Error;
    use tower::ServiceExt;

    use crate::result::{ApiError, ApiResult, ErrorCode};

    #[must_use]
    fn build_app() -> Router {
        // this triggers Axum's built-in extractor rejection.
        async fn axum_emitted_error_handler(_: Json<ApiError>) -> &'static str {
            unreachable!()
        }

        async fn custom_error_handler() -> Result<String, ApiError> {
            Err(ApiError {
                code: ErrorCode::InvalidRequest,
                message: Cow::Borrowed("Why"),
                request_id: None,
            })
        }

        #[derive(Debug, Error)]
        #[error("IO error occurred")]
        struct IoError;

        async fn io_error_handler() -> ApiResult<String> {
            let report = Report::new(std::io::Error::new(std::io::ErrorKind::Other, "Oh no!"))
                .change_context(IoError);

            Err(report)?;
            unreachable!()
        }

        Router::new()
            .route("/axum_extractor_error", get(axum_emitted_error_handler))
            .route("/custom_error", get(custom_error_handler))
            .route("/io_error", get(io_error_handler))
            .layer(axum::middleware::from_fn(super::middleware))
    }

    async fn send(method: Method, path: &str) -> (Parts, Bytes) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();

        let (parts, body) = build_app().oneshot(req).await.unwrap().into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();

        (parts, bytes)
    }

    fn expected_json(error: &ApiError) -> Bytes {
        Bytes::from(serde_json::to_string(error).unwrap())
    }

    #[tokio::test]
    async fn axum_extractor_error_becomes_json() {
        eden_common::testing::init();

        let (parts, bytes) = send(Method::GET, "/axum_extractor_error").await;
        assert_eq!(parts.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let expected = expected_json(&ApiError {
            code: ErrorCode::InvalidRequest,
            message: Cow::Borrowed("Expected request with `Content-Type: application/json`"),
            request_id: None,
        });
        assert_eq!(bytes, expected);
    }

    #[tokio::test]
    async fn custom_error_should_be_preserved() {
        let (parts, bytes) = send(Method::GET, "/custom_error").await;
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);

        let expected = expected_json(&ApiError {
            code: ErrorCode::InvalidRequest,
            message: Cow::Borrowed("Why"),
            request_id: None,
        });
        assert_eq!(bytes, expected);
    }

    #[tokio::test]
    async fn unhandled_report_should_be_an_internal_error() {
        let (parts, bytes) = send(Method::GET, "/io_error").await;
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);

        let expected = expected_json(&ApiError::INTERNAL);
        assert_eq!(bytes, expected);
    }
}
