use eden_kernel::Kernel;
use error_stack::{Report, ResultExt};
use std::{net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::net::TcpListener;

pub mod controllers;
pub mod errors;
pub mod middleware;
pub mod model;
pub mod router;

#[derive(Debug, Error)]
pub enum GatewayServerError {
    #[error("Could not bind server address")]
    Bind,

    #[error("Error occurred while trying to serve a gateway server")]
    Serving,
}

#[tracing::instrument(skip_all, name = "server.run")]
pub async fn service(kernel: Arc<Kernel>) -> Result<(), Report<GatewayServerError>> {
    let listener = TcpListener::bind(("127.0.0.1", 8080))
        .await
        .change_context(GatewayServerError::Bind)?;

    let router = crate::router::build(kernel.clone());
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
