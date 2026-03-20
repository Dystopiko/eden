use axum::extract::{FromRequest, FromRequestParts, Json, Request};
use axum_extra::either::Either;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use validator::Validate;

use crate::errors::ApiError;

/// Newtype wrapper around [`eden_kernel::Kernel`] for use as an Axum extractor.
///
/// Clones the inner [`Arc`] so each handler receives its own handle without
/// the boilerplate of a manual [`Extension`] extraction.
pub struct Kernel(pub Arc<eden_core::Kernel>);

impl FromRequestParts<Arc<eden_core::Kernel>> for Kernel {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &Arc<eden_core::Kernel>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Kernel(state.clone()))
    }
}

pub struct Validated<T>(pub T);

impl<S, T> FromRequest<S> for Validated<Json<T>>
where
    Json<T>: FromRequest<S>,
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Either<<Json<T> as FromRequest<S>>::Rejection, ApiError>;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let json = <Json<T> as FromRequest<S>>::from_request(req, state)
            .await
            .map_err(Either::E1)?;

        json.validate()
            .map_err(ApiError::from_validate)
            .map_err(Either::E2)?;

        Ok(Self(json))
    }
}

impl<'de, T: serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for Validated<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self)
    }
}

impl<T> std::ops::Deref for Validated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
