use eden_kernel::Kernel;
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, prelude::*};

/// Checks the database configuration to ensure data persistence.
///
/// If an in-memory database is detected without explicit user consent,
/// the application will log a critical error and initiate a shutdown.
pub async fn check_database(kernel: Arc<Kernel>) {
    let accepts_data_loss = eden_common::env::var("EDEN_I_ACCEPT_DATA_LOSS")
        .ok()
        .flatten()
        .is_some();

    if kernel.config.database.primary.url.is_memory() && !accepts_data_loss {
        tracing::error!(
            "Primary database is configured to be stored in memory!\n\
            ========================================================================================\n\
            You are currently using an in-memory database. All data will be permanently\n\
            lost when the application shuts down.\n\
            \n\
            Action required:\n\
            - Configure a persistent database in the config file (use the generated config file if it shows `wrote default config log`).\n\
            - or, if this is intentional, set `EDEN_I_ACCEPT_DATA_LOSS=true`.\n\
            ========================================================================================"
        );
        kernel.shutdown_signal.initiate();
    } else if accepts_data_loss {
        tracing::warn!(
            "Running with an in-memory database (`EDEN_I_ACCEPT_DATA_LOSS` is enabled). \
            All data will be permanently wiped upon restart. By utilizing this configuration, \
            you acknowledge and agree that you are solely liable for any resulting data loss."
        );
    }
}

/// Initializes the global [`tracing`] subscriber.
pub fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_filter(env_filter);

    tracing_subscriber::registry().with(fmt_layer).init();
}

/// Waits for an OS-level shutdown signal and returns the signal name
/// when one is received.
///
/// | Platform | Signals handled        |
/// |----------|------------------------|
/// | Unix     | `SIGINT`, `SIGTERM`    |
/// | Windows  | Ctrl-C console event   |
///
/// This function is intended to be used as a graceful-shutdown trigger.
#[must_use]
pub async fn shutdown_signal() -> &'static str {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

        tokio::select! {
            _ = sigint.recv() => "SIGINT",
            _ = sigterm.recv() => "SIGTERM",
        }
    }

    #[cfg(windows)]
    {
        use tokio::signal::windows::ctrl_c;

        let mut ctrl_c = ctrl_c().expect("failed to install Ctrl-C handler");
        ctrl_c.recv().await;

        "CTRL+C"
    }
}
