use eden_core::Kernel;
use error_stack::{Report, ResultExt};
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::net::TcpListener;

pub mod controllers;
pub mod errors;
pub mod extract;
pub mod middleware;
pub mod ratelimiter;
pub mod router;

pub use self::errors::ApiError;
pub use self::ratelimiter::RateLimiter;

#[derive(Debug, Error)]
pub enum GatewayServerError {
    #[error("could not bind server address")]
    Bind,

    #[error("error occurred while trying to serve a gateway server")]
    Serving,
}

#[tracing::instrument(skip_all, name = "server.run")]
pub async fn service(kernel: Arc<Kernel>) -> Result<(), Report<GatewayServerError>> {
    let config = &kernel.config.gateway;
    let listener = TcpListener::bind((config.ip, config.port))
        .await
        .change_context(GatewayServerError::Bind)?;

    let ratelimiter = RateLimiter::new(HashMap::new());
    let router = crate::router::build(kernel.clone(), ratelimiter);

    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let addr = listener
        .local_addr()
        .change_context(GatewayServerError::Bind)?;

    tracing::info!("listening at http://{addr}");
    axum::serve(listener, make_service)
        .with_graceful_shutdown(async move { kernel.shutdown_signal.subscribe().await })
        .await
        .change_context(GatewayServerError::Serving)?;

    tracing::info!("server has gracefully shutdown");
    Ok(())
}
