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

    let start = Instant::now();
    let response = next.run(request).await;
    metrics.requests_total.inc();

    let endpoint = match matched_path {
        Some(ref p) => p.as_str(),
        None => "<unknown>",
    };

    metrics
        .response_times
        .with_label_values(&[endpoint])
        .observe(start.elapsed().as_micros() as f64 / 1_000_000.0);

    response
}
