use axum_server::tls_rustls::{RustlsAcceptor, RustlsConfig};
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

    #[error("failed to setup TLS")]
    SetupTLS,

    #[error("error occurred while trying to serve a gateway server")]
    Serving,
}

#[tracing::instrument(skip_all, name = "server.run")]
pub async fn service(kernel: Arc<Kernel>) -> Result<(), Report<GatewayServerError>> {
    // axum_server only accepts std's TcpListener
    let config = &kernel.config.gateway;
    let listener = TcpListener::bind((config.ip, config.port))
        .await
        .and_then(|v| v.into_std())
        .change_context(GatewayServerError::Bind)?;

    let ratelimiter = RateLimiter::new(HashMap::new());
    let router = crate::router::build(kernel.clone(), ratelimiter);

    let rustls_config =
        RustlsConfig::from_pem_file(&config.tls_cert_pem, &config.tls_private_key_pem)
            .await
            .change_context(GatewayServerError::SetupTLS)?;

    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let addr = listener
        .local_addr()
        .change_context(GatewayServerError::Bind)?;

    tracing::info!("listening at https://{addr}");

    let tls_acceptor = RustlsAcceptor::new(rustls_config);
    let handle = axum_server::Handle::new();

    let server = handle.clone();
    tokio::spawn(async move {
        kernel.shutdown_signal.subscribe().await;
        server.graceful_shutdown(None);
    });

    axum_server::from_tcp(listener)
        .change_context(GatewayServerError::Bind)?
        .acceptor(tls_acceptor)
        .handle(handle.clone())
        .serve(make_service)
        .await
        .change_context(GatewayServerError::Serving)?;

    tracing::info!("server has gracefully shutdown");
    Ok(())
}
