use eden_common::signals::ShutdownSignal;
use eden_config::{Config, EditableConfig, error::ConfigLoadError};
use eden_kernel::Kernel;

use error_stack::{Report, ResultExt};
use std::{net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::net::TcpListener;

#[derive(Debug, Error)]
#[error("Could not start gateway server")]
struct ServerError;

fn main() -> Result<(), Report<ServerError>> {
    let dotenv = eden_common::env::load().ok().flatten();
    eden_common::bootstrap::init_tracing();
    eden_common::bootstrap::init_rustls().change_context(ServerError)?;

    if let Some(dotenv) = dotenv {
        tracing::debug!("using dotenv file: {}", dotenv.display());
    }

    let config = load_config().change_context(ServerError)?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .change_context(ServerError)
        .attach("while trying to build tokio runtime")?;

    let kernel = rt.block_on(async {
        let built = Kernel::builder()
            .pools(&config.database)
            .change_context(ServerError)?
            .config(Arc::new(config))
            .shutdown_signal(ShutdownSignal::new())
            .build();

        Ok::<_, Report<ServerError>>(built)
    })?;

    let router = eden_gateway_server::router::build(kernel.clone());
    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();

    rt.block_on(async {
        let listener = TcpListener::bind(("127.0.0.1", 8080)).await?;
        let addr = listener.local_addr()?;
        tracing::info!("listening at http://{addr}");

        let shutdown_signal = kernel.shutdown_signal.clone();
        tokio::spawn(async move {
            let signal = eden_common::bootstrap::shutdown_signal().await;
            tracing::warn!("received {signal}; initiating graceful shutdown");
            shutdown_signal.initiate();
        });

        axum::serve(listener, make_service)
            .with_graceful_shutdown(async move { kernel.shutdown_signal.subscribe().await })
            .await
    })
    .change_context(ServerError)?;

    tracing::info!("server has gracefully shutdown");
    Ok(())
}

#[tracing::instrument(name = "config.load", level = "debug")]
fn load_config() -> Result<Config, Report<ConfigLoadError>> {
    let path = Config::suggest_path();

    let mut config = EditableConfig::new(&path);
    if !config.exists() {
        config.save().change_context(ConfigLoadError)?;
        tracing::info!("wrote default config at: {}", path.display());
    } else {
        config.reload()?;
    }

    tracing::debug!(config = ?&*config, "using config file: {}", config.path().display());
    Ok(config.into_inner())
}
