use axum::extract::{Extension, Json};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{Router, get, post, put};
use eden_core::Kernel;
use std::sync::Arc;

use crate::{controllers::*, errors::ApiError, ratelimiter::RateLimiter};

pub fn build(kernel: Arc<Kernel>, ratelimiter: Arc<RateLimiter>) -> Router<()> {
    let router = Router::new()
        .route("/", get(index))
        .route(
            "/admin/members/{id}/invitees",
            get(admin::members::invitees::invitees),
        )
        .route(
            "/admin/members/{id}",
            get(admin::members::get)
                .patch(admin::members::patch)
                .post(admin::members::post),
        )
        .route(
            "/admin/settings",
            get(admin::settings::get).patch(admin::settings::patch),
        )
        .route(
            "/alerts/admin_commands",
            put(alerts::admin_commands::publish),
        )
        .route("/members/link/minecraft", post(members::link::minecraft))
        .route("/metrics", get(metrics::prometheus))
        .route("/sessions/validate", post(sessions::validate::validate))
        .route("/sessions", post(sessions::post::post))
        .layer(Extension(ratelimiter));

    let router = router
        .fallback(async |method: Method| match method {
            Method::HEAD => StatusCode::NOT_FOUND.into_response(),
            _ => Json(ApiError::NOT_FOUND).into_response(),
        })
        .with_state(kernel.clone());

    crate::middleware::apply(kernel, router)
}
