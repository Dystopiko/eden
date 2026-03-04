use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use eden_sqlite::error::{PoolError, SqlErrorType};
use std::sync::Arc;

use crate::result::{ApiError, ErasedReport};

pub async fn middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;

    let status = res.status();
    if status.is_client_error() || status.is_server_error() {
        // Try to get the erased report through extensions
        let report = res
            .extensions_mut()
            .remove::<Arc<ErasedReport>>()
            .and_then(Arc::into_inner);

        if let Some(report) = report {
            res = transform_into_api_error(report).into_response();
        } else {
            res = jsonify_error(res).await;
        }
    }

    res
}

async fn jsonify_error(res: Response) -> Response {
    debug_assert!(res.status().is_client_error() || res.status().is_server_error());

    let (parts, body) = res.into_parts();

    let bytes = axum::body::to_bytes(body, 1_000_000).await.unwrap();
    if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
        return (parts, error).into_response();
    }

    // Otherwise, we'll just emit internal error instead?
    (parts, bytes).into_response()
}

fn transform_into_api_error(report: ErasedReport) -> ApiError {
    if let Some(error) = report.downcast_ref::<PoolError>() {
        match error {
            PoolError::General => {
                tracing::error!(error = ?report, "encountered a pool error");
                return ApiError::INTERNAL;
            }
            PoolError::Unhealthy => return ApiError::SERVICE_UNAVAILABLE,
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
    use std::borrow::Cow;

    use axum::body::{Body, Bytes};
    use axum::extract::Json;
    use axum::http::{Method, Request, StatusCode, response::Parts};
    use axum::routing::{Router, get};
    use error_stack::Report;
    use tower::ServiceExt;

    use crate::result::{ApiError, ApiResult};

    fn build_app() -> Router {
        async fn axum_emitted_error(error: Json<ApiError>) -> &'static str {
            unreachable!()
        }

        async fn custom_error() -> Result<String, ApiError> {
            Err(ApiError {
                code: crate::result::ApiErrorCode::InvalidRequest,
                message: Cow::Borrowed("Why"),
                request_id: None,
            })
        }

        async fn io_error() -> ApiResult<String> {
            let error = Report::new(std::io::Error::new(std::io::ErrorKind::Other, "Oh no!"));
            Err(error.into())
        }

        Router::new()
            .route("/axum_emitted_error", get(axum_emitted_error))
            .route("/io_error", get(io_error))
            .route("/custom", get(custom_error))
            .layer(axum::middleware::from_fn(super::middleware))
    }

    async fn request(method: Method, path: &str) -> (Parts, Bytes) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();

        let response = build_app().oneshot(request).await.unwrap();
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        (parts, bytes)
    }

    #[tokio::test]
    async fn test_axum_emitted_error() {
        let (parts, bytes) = request(Method::GET, "/axum_emitted_error").await;
        assert_eq!(parts.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let expected = Bytes::from(
            serde_json::to_string(&ApiError {
                code: crate::result::ApiErrorCode::InvalidRequest,
                message: Cow::Borrowed("Why"),
                request_id: None,
            })
            .unwrap(),
        );
        pretty_assertions::assert_eq!(bytes, expected);
    }

    #[tokio::test]
    async fn test_custom_error() {
        let (parts, bytes) = request(Method::GET, "/custom").await;
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);

        let expected = Bytes::from(
            serde_json::to_string(&ApiError {
                code: crate::result::ApiErrorCode::InvalidRequest,
                message: Cow::Borrowed("Why"),
                request_id: None,
            })
            .unwrap(),
        );
        pretty_assertions::assert_eq!(bytes, expected);
    }

    #[tokio::test]
    async fn test_io_error() {
        let (parts, bytes) = request(Method::GET, "/io_error").await;
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);

        let expected = Bytes::from(serde_json::to_string(&ApiError::INTERNAL).unwrap());
        pretty_assertions::assert_eq!(bytes, expected);
    }
}
