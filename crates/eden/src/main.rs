use eden_common::signals::ShutdownSignal;
use eden_config::{Config, EditableConfig, error::ConfigLoadError};
use eden_kernel::Kernel;
use erased_report::ErasedReport;
use error_stack::{Report, ResultExt};
use std::sync::Arc;

fn main() -> Result<(), ErasedReport> {
    let dotenv = eden_common::env::load().ok().flatten();
    eden_common::bootstrap::init_rustls()?;
    eden::bootstrap::init_tracing();

    if let Some(dotenv) = dotenv {
        tracing::debug!("using dotenv file: {}", dotenv.display());
    }

    let config = load_config()?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .attach("while trying to build tokio runtime")?;

    let kernel = rt.block_on(async {
        let built = Kernel::builder()
            .discord_from_config(&config.bot)
            .pools(&config.database)?
            .config(Arc::new(config))
            .shutdown_signal(ShutdownSignal::new())
            .build();

        Ok::<_, ErasedReport>(built)
    })?;

    rt.block_on(async {
        let gateway = eden_gateway_server::service(kernel.clone());

        let shutdown_signal = kernel.shutdown_signal.clone();
        tokio::spawn(async move {
            let signal = eden::bootstrap::shutdown_signal().await;
            tracing::warn!("received {signal}; initiating graceful shutdown");
            shutdown_signal.initiate();
        });

        gateway.await
    })?;

    tracing::info!("closing down Eden");
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
