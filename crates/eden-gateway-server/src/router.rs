use axum::extract::Json;
use axum::http::{Method, StatusCode};
use axum::middleware::from_fn;
use axum::response::IntoResponse;
use axum::routing::{Router, get, post};
use eden_kernel::Kernel;
use std::sync::Arc;

use crate::controllers::*;
use crate::errors::ApiError;
use crate::middleware::{extract_client_ip, normalize_error, trace_request};

#[must_use]
pub fn build(kernel: Arc<Kernel>) -> Router<()> {
    let middleware = tower::ServiceBuilder::new()
        .layer(from_fn(extract_client_ip::middleware))
        .layer(from_fn(trace_request::middleware))
        .layer(from_fn(normalize_error::middleware));

    let router = Router::new()
        .route("/", get(index))
        .route("/sessions", post(sessions::try_grant));

    router
        .layer(middleware)
        .fallback(async |method: Method| match method {
            Method::HEAD => StatusCode::NOT_FOUND.into_response(),
            _ => Json(ApiError::NOT_FOUND).into_response(),
        })
        .with_state(kernel)
}
