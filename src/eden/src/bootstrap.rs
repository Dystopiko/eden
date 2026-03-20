use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, prelude::*};

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
