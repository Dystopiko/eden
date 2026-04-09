use std::time::Instant;

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};

use crate::extract::Kernel;

pub async fn middleware(
    Kernel(kernel): Kernel,
    matched_path: Option<MatchedPath>,
    request: Request,
    next: Next,
) -> Response {
    let Some(metrics) = kernel.metrics.as_ref() else {
        return next.run(request).await;
    };

    let method = request.method().as_str().to_string();
    let start = Instant::now();

    let response = next.run(request).await;
    metrics.requests_total.inc();

    let endpoint = match matched_path {
        Some(ref p) => p.as_str(),
        None => "<unknown>",
    };

    metrics
        .response_times
        .with_label_values(&[endpoint, &method])
        .observe(start.elapsed().as_secs_f64());

    response
}
