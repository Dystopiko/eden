use axum::{extract::Request, http::header, middleware::Next, response::IntoResponse};

use crate::{ApiError, extract::Kernel};

pub async fn middleware(Kernel(kernel): Kernel, request: Request, next: Next) -> impl IntoResponse {
    let Some(token) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return ApiError::ACCESS_DENIED.into_response();
    };

    let answer = &kernel.config.gateway.shared_secret_token;

    // SECURITY: <PartialEq as SharedSecretToken> uses constant_time_eq
    if !answer.eq(&token) {
        return ApiError::ACCESS_DENIED.into_response();
    }

    next.run(request).await
}
