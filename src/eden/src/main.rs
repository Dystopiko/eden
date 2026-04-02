use eden_background_worker::Runner;
use eden_config::{Config, EditableConfig, error::ConfigLoadError};
use eden_core::{
    jobs::{JobContext, RunnerExt},
    kernel::Kernel,
};
use eden_utils::signals::ShutdownSignal;
use erased_report::ErasedReport;
use error_stack::{Report, ResultExt};
use futures::{FutureExt, TryFutureExt};
use rand::seq::IteratorRandom;
use std::{path::Path, sync::Arc};

fn main() -> Result<(), ErasedReport> {
    let dotenv = eden_utils::env::load().ok().flatten();
    eden_utils::bootstrap::init_rustls()?;
    eden::bootstrap::init_tracing();

    if let Some(dotenv) = dotenv {
        tracing::debug!("using dotenv file: {}", dotenv.display());
    }

    let config = load_config()?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let kernel = rt.block_on(async {
        let token = config.bot.token.as_str().to_string();
        let http = Arc::new(twilight_http::Client::builder().token(token).build());

        let built = Kernel::builder()
            .pools_from_config(&config.database)?
            .config(Arc::new(config))
            .http(http)
            .shutdown_signal(ShutdownSignal::new())
            .build();

        Ok::<_, ErasedReport>(built)
    })?;

    // Perform database migrations for the primary pool only.
    rt.block_on(eden_database::migrations::perform(
        kernel.pools.primary_db(),
    ))?;

    let _sentry = eden_sentry::init(kernel.config.sentry.as_ref());
    if _sentry.is_some() {
        tracing::info!("Sentry integration is enabled");
    } else {
        tracing::info!("Sentry integration is disabled");
    }

    let result: Result<(), ErasedReport> = rt.block_on(async {
        let job_context = JobContext::builder()
            .discord(kernel.http.clone())
            .kernel(kernel.clone())
            .build();

        let workers_handle = Runner::new(job_context, kernel.pools.clone())
            .register_core_job_types()
            .workers(1)
            .start();

        let discord = if kernel.config.bot.enabled {
            eden_discord_bot::service(kernel.clone(), kernel.http.clone())
                .map_err(|report| ErasedReport::from_report(report))
                .boxed()
        } else {
            tracing::warn!("Discord service is disabled");
            async { Ok::<(), ErasedReport>(()) }.boxed()
        };

        let shutdown_signal = kernel.shutdown_signal.clone();
        let gateway = eden_gateway_server::service(kernel.clone())
            .inspect_err(|_| {
                tracing::warn!("gateway service failed! initiating shutdown");
                shutdown_signal.initiate();
            })
            .map_err(|report| ErasedReport::from_report(report));

        let shutdown_signal = kernel.shutdown_signal.clone();
        tokio::spawn(async move {
            let signal = eden::bootstrap::shutdown_signal().await;
            tracing::warn!("received {signal}; initiating graceful shutdown");
            shutdown_signal.initiate();
        });

        let result = tokio::try_join!(discord, gateway);
        workers_handle.shutdown().await;

        result.map(|_| ())
    });

    tracing::info!("closing down Eden");
    result
}

#[tracing::instrument(name = "config.load", level = "debug")]
fn load_config() -> Result<Config, Report<ConfigLoadError>> {
    let Some(path) = Config::find() else {
        // Save the template config file into the current directory
        EditableConfig::save_template(Config::FILE_NAME)
            .change_context(ConfigLoadError)
            .attach("while trying to save template config file")?;

        let template_path = Path::new(Config::FILE_NAME);
        tracing::warn!(
            "No config file found! Wrote template config file at: {}",
            template_path.display()
        );
        tracing::warn!("Please edit eden.toml to configure Eden then re-run");

        std::process::exit(1);
    };

    let mut config = EditableConfig::open(path)?;

    // Generate shared secret token if needed.
    if config.gateway.shared_secret_token.as_str().is_empty() {
        tracing::warn!(
            "gateway.shared_secret_token is empty. Generating new one (this will invalidate the previous token)..."
        );

        config
            .edit(|_, document| {
                if let Some(gateway) = document
                    .entry("gateway")
                    .or_insert(toml_edit::table())
                    .as_table_like_mut()
                {
                    gateway
                        .entry("shared_secret_token")
                        .or_insert_with(|| toml_edit::value(generate_shared_token()));
                }
            })
            .change_context(ConfigLoadError)?;
    }

    tracing::debug!(config = ?&*config, "using config file: {}", config.path().display());
    Ok(config.into_inner())
}

const GENERATED_TOKEN_LENGTH: usize = 64;
static TOKEN_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcdefghijklmnopqrstuvwxyz_-";

fn generate_shared_token() -> String {
    let mut rng = rand::rng();
    let mut token = String::with_capacity(GENERATED_TOKEN_LENGTH);

    let chars = TOKEN_CHARS.chars();
    for _ in 0..GENERATED_TOKEN_LENGTH {
        let c = chars
            .clone()
            .choose(&mut rng)
            .expect("should generate a random letter");

        token.push(c);
    }

    token
}
