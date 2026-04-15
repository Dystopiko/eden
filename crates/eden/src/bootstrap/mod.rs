use eden_config::Config;
use toml_edit::DocumentMut;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, prelude::*};

mod generate;

/// Provides missing values for the [Eden config].
///
/// [Eden config]: eden_config::Config
pub fn provide_defaults_for_config(config: &Config, document: &mut DocumentMut) {
    if config.gateway.shared_secret_token.as_str().is_empty() {
        let field = document
            .entry("gateway")
            .or_insert(toml_edit::table())
            .as_table_like_mut()
            .expect("config is already parsed. safe to assume gateway is a table")
            .entry("shared_secret_token")
            .or_insert_with(|| toml_edit::value(""));

        tracing::warn!(
            target: "eden",
            "gateway.shared_secret_token is empty. Generating new one \
            (this will invalidate the previous token)..."
        );
        *field = toml_edit::value(self::generate::shared_token());
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

    let sentry_layer = sentry::integrations::tracing::layer()
        .enable_span_attributes()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(sentry_layer)
        .init();
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
