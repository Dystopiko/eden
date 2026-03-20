use axum::{Router, middleware::from_fn};

pub mod extract_client_ip;
pub mod normalize_error;
pub mod trace_request;

pub fn apply(router: Router<()>) -> Router<()> {
    let middleware = tower::ServiceBuilder::new()
        .layer(from_fn(extract_client_ip::middleware))
        .layer(from_fn(trace_request::middleware))
        .layer(from_fn(normalize_error::middleware));

    router.layer(middleware)
}
