use axum::extract::Json;
use serde_json::{Value, json};

pub mod alerts;
pub mod sessions;

pub async fn index() -> Json<Value> {
    Json(json!({ "hello": "world" }))
}

/// The standard `Result` type for API handlers.
pub type ApiResult<T> = std::result::Result<T, crate::errors::ApiError>;
