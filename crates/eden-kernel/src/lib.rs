use bon::Builder;
use eden_common::signals::ShutdownSignal;
use eden_config::Config;
use eden_database::DatabasePools;
use eden_sqlite::{Pool, error::PoolBuildError};
use error_stack::{Report, ResultExt};
use std::sync::Arc;

#[derive(Debug, Builder)]
#[builder(finish_fn(name = "build_inner", vis = ""))]
pub struct Kernel {
    /// App configuration
    pub config: Arc<Config>,

    /// Discord Rest API client
    pub discord: Arc<twilight_http::Client>,

    /// Database connection pool connected to the primary database
    pub primary_db: Pool,

    /// Database connection pool connected to the read-only replica database
    pub replica_db: Option<Pool>,

    /// Shutdown signal to notify all services
    pub shutdown_signal: ShutdownSignal,
}

impl<S: kernel_builder::State> KernelBuilder<S> {
    /// Initializes Discord HTTP client field lazily from the provided
    /// [Discord bot configuration].
    ///
    /// [Discord bot configuration]: eden_config::sections::Bot
    pub fn discord_from_config(
        self,
        config: &eden_config::sections::Bot,
    ) -> KernelBuilder<kernel_builder::SetDiscord<S>>
    where
        S::Discord: kernel_builder::IsUnset,
    {
        let discord = twilight_http::Client::builder()
            .token(config.token.as_str().to_string())
            .build();

        self.discord(Arc::new(discord))
    }

    /// Initializes `primary_db` and `replica_db` fields lazily from the
    /// provided [database configuration].
    ///
    /// [database configuration]: eden_config::sections::Database
    pub fn pools(
        self,
        config: &eden_config::sections::Database,
    ) -> Result<
        KernelBuilder<kernel_builder::SetReplicaDb<kernel_builder::SetPrimaryDb<S>>>,
        Report<PoolBuildError>,
    >
    where
        S::PrimaryDb: kernel_builder::IsUnset,
        S::ReplicaDb: kernel_builder::IsUnset,
    {
        let primary_db = pool_from_config(&config.primary)
            .attach("while trying to create primary database pool")?;

        let replica_db = config
            .replica
            .as_ref()
            .map(pool_from_config)
            .transpose()
            .attach("while trying to create replica database pool")?;

        Ok(self.primary_db(primary_db).maybe_replica_db(replica_db))
    }

    /// Creates a new [`Kernel`] and wraps it in an [`Arc`] for shared ownership.
    #[must_use]
    pub fn build(self) -> Arc<Kernel>
    where
        S: kernel_builder::IsComplete,
    {
        Arc::new(self.build_inner())
    }
}

impl DatabasePools for Kernel {
    fn primary_db(&self) -> &eden_sqlite::Pool {
        &self.primary_db
    }

    fn replica_db(&self) -> Option<&eden_sqlite::Pool> {
        self.replica_db.as_ref()
    }
}

fn pool_from_config(
    config: &eden_config::sections::DatabasePool,
) -> Result<Pool, Report<PoolBuildError>> {
    let config = eden_sqlite::PoolConfig::builder()
        .min_connections(config.min_connections)
        .max_connections(config.max_connections.get())
        .readonly(config.readonly)
        .url(config.url.as_str().into())
        .build()
        .expect("max_connections has non-zero u32 type");

    Pool::new(config)
}
