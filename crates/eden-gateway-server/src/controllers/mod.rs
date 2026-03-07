use axum::extract::{FromRequestParts, Json};
use serde_json::{Value, json};
use std::sync::Arc;

pub mod alert;
pub mod session;

pub async fn index() -> Json<Value> {
    Json(json!({ "hello": "world" }))
}

/// The standard `Result` type for API handlers.
pub type ApiResult<T> = std::result::Result<T, crate::errors::ApiError>;

/// Newtype wrapper around [`eden_kernel::Kernel`] for use as an Axum extractor.
///
/// Clones the inner [`Arc`] so each handler receives its own handle without
/// the boilerplate of a manual [`Extension`] extraction.
pub struct Kernel(pub Arc<eden_kernel::Kernel>);

impl FromRequestParts<Arc<eden_kernel::Kernel>> for Kernel {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &Arc<eden_kernel::Kernel>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Kernel(state.clone()))
    }
}
