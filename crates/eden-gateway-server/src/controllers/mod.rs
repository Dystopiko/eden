use axum::extract::{FromRequestParts, Json};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;

pub mod sessions;

pub async fn index() -> Json<Value> {
    Json(json!({ "hello": "world" }))
}

/// A wrapper to the actual [`eden_kernel::Kernel`] object, this is to easily
/// extract the [`Kernel`] if needed by a controller/route.
pub struct Kernel(pub Arc<eden_kernel::Kernel>);

impl FromRequestParts<Arc<eden_kernel::Kernel>> for Kernel {
    type Rejection = Infallible;

    fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        inner: &Arc<eden_kernel::Kernel>,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async { Ok(Kernel(inner.clone())) }
    }
}
