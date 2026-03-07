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
            .discord_cache(eden_discord_bot::default_in_memory_cache())
            .discord_from_config(&config.bot)
            .pools(&config.database)?
            .config(Arc::new(config))
            .shutdown_signal(ShutdownSignal::new())
            .build();

        Ok::<_, ErasedReport>(built)
    })?;

    let result: Result<(), ErasedReport> = rt.block_on(async {
        tokio::spawn(eden::bootstrap::check_database(kernel.clone()));

        let bot = eden_discord_bot::service(kernel.clone());
        let gateway = eden_gateway_server::service(kernel.clone());

        let shutdown_signal = kernel.shutdown_signal.clone();
        tokio::spawn(async move {
            let signal = eden::bootstrap::shutdown_signal().await;
            tracing::warn!("received {signal}; initiating graceful shutdown");
            shutdown_signal.initiate();
        });

        let (bot, gateway) = tokio::join!(bot, gateway);
        bot?;
        gateway?;

        Ok(())
    });

    tracing::info!("closing down Eden");
    result
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
