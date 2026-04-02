use axum::{
    Router,
    http::StatusCode,
    middleware::{from_fn, from_fn_with_state},
};
use eden_core::Kernel;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use std::{sync::Arc, time::Duration};
use tower_http::timeout::{RequestBodyTimeoutLayer, TimeoutLayer};

pub mod extract_client_ip;
pub mod normalize_error;
pub mod requires_auth;
pub mod trace_request;

pub fn apply(kernel: Arc<Kernel>, router: Router<()>) -> Router<()> {
    let sentry_middleware = tower::ServiceBuilder::new()
        .layer(NewSentryLayer::new_from_top())
        .layer(SentryHttpLayer::new());

    let middleware = tower::ServiceBuilder::new()
        .layer(from_fn(extract_client_ip::middleware))
        .layer(from_fn(trace_request::middleware))
        .layer(from_fn_with_state(kernel, requires_auth::middleware))
        .layer(from_fn(normalize_error::middleware));

    router
        .layer(middleware)
        .layer(sentry_middleware)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(30)))
}
