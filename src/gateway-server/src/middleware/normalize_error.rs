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
    errors::{ApiError, ErrorCode},
    middleware::trace_request::RequestId,
};

#[derive(Debug, Error)]
#[error("failed to convert plain-text error body to JSON")]
struct JsonifyError;

/// Normalizes all error responses into a consistent [`ApiError`] JSON payload.
pub async fn middleware(request: Request, next: Next) -> impl IntoResponse {
    let mut response = next.run(request).await;

    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    let request_id = response.extensions().get::<RequestId>().map(|v| v.0);
    if let Some(report) = take_erased_report(&mut response) {
        return crate::errors::classify(report)
            .maybe_request_id(request_id)
            .into_response();
    }

    match jsonify_plain_text_error(response, request_id).await {
        Ok(converted) => converted,
        Err(report) => {
            tracing::error!(error = ?report, "failed to jsonify plain-text error response");
            eden_sentry::capture_report(&report);

            ApiError::INTERNAL
                .maybe_request_id(request_id)
                .into_response()
        }
    }
}

fn take_erased_report(res: &mut Response) -> Option<ErasedReport> {
    res.extensions_mut()
        .remove::<Arc<ErasedReport>>()
        .and_then(Arc::into_inner)
}

async fn jsonify_plain_text_error(
    res: Response,
    request_id: Option<Uuid>,
) -> Result<Response, Report<JsonifyError>> {
    const MAX_ERROR_BODY_BYTES: usize = 1_000_000;

    let (mut parts, body) = res.into_parts();
    if !is_plain_text_utf8(&parts.headers) {
        return Ok((parts, body).into_response());
    }

    let bytes = axum::body::to_bytes(body, MAX_ERROR_BODY_BYTES)
        .await
        .change_context(JsonifyError)?;

    let message = String::from_utf8(bytes.into()).change_context(JsonifyError)?;
    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);

    let error = ApiError {
        code: ErrorCode::InvalidRequest,
        message: Cow::Owned(message),
        headers: None,
        request_id,
        report: None,
    };

    Ok((parts, Json(error)).into_response())
}

fn is_plain_text_utf8(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Mime::from_str(v).ok())
        .is_some_and(|mime| mime == mime::TEXT_PLAIN_UTF_8)
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

    use crate::{
        controllers::ApiResult,
        errors::{ApiError, ErrorCode},
    };

    fn build_app() -> Router {
        async fn axum_extractor_rejection(_: Json<ApiError>) -> &'static str {
            unreachable!()
        }

        async fn known_error_handler() -> ApiResult<String> {
            Err(ApiError::from_static(ErrorCode::InvalidRequest, "Why"))
        }

        #[derive(Debug, Error)]
        #[error("IO error")]
        struct IoError;

        async fn unhandled_error_handler() -> ApiResult<String> {
            Err(Report::new(std::io::Error::other("Oh no!")).change_context(IoError))?;
            unreachable!()
        }

        Router::new()
            .route("/axum_extractor_error", get(axum_extractor_rejection))
            .route("/known_error", get(known_error_handler))
            .route("/unhandled_error", get(unhandled_error_handler))
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

    fn as_json(error: &ApiError) -> Bytes {
        Bytes::from(serde_json::to_string(error).unwrap())
    }

    #[tokio::test]
    async fn axum_extractor_error_becomes_json() {
        eden_utils::testing::init();

        let (parts, bytes) = send(Method::GET, "/axum_extractor_error").await;
        assert_eq!(parts.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            bytes,
            as_json(&ApiError {
                code: ErrorCode::InvalidRequest,
                message: Cow::Borrowed("Expected request with `Content-Type: application/json`"),
                headers: None,
                report: None,
                request_id: None,
            })
        );
    }

    #[tokio::test]
    async fn known_error_is_preserved() {
        eden_utils::testing::init();

        let (parts, bytes) = send(Method::GET, "/known_error").await;
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            bytes,
            as_json(&ApiError {
                code: ErrorCode::InvalidRequest,
                message: Cow::Borrowed("Why"),
                headers: None,
                report: None,
                request_id: None,
            })
        );
    }

    #[tokio::test]
    async fn unhandled_report_becomes_internal_error() {
        eden_utils::testing::init();

        let (parts, bytes) = send(Method::GET, "/unhandled_error").await;
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(bytes, as_json(&ApiError::INTERNAL));
    }
}
