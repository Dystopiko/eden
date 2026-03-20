use bon::Builder;
use eden_background_worker::{BackgroundJob, background_job::EnqueueJobError};
use eden_config::{Config, sections::minecraft::UuidOrUsername};
use eden_database::{DatabasePools, views::McAccountView};
use eden_sqlite::{Pool, error::PoolBuildError};
use eden_utils::signals::ShutdownSignal;
use error_stack::{Report, ResultExt};
use std::{collections::HashSet, fmt, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Builder)]
#[builder(finish_fn(name = "build_inner", vis = ""))]
pub struct Kernel {
    /// App configuration
    pub config: Arc<Config>,

    /// Database connection pool connected to either primary or
    /// primary+replica databases
    pub pools: DatabasePools,

    /// Shutdown signal to notify all services
    pub shutdown_signal: ShutdownSignal,
}

impl Kernel {
    #[must_use]
    pub fn resolve_mc_perks(&self, view: &McAccountView) -> Vec<String> {
        let config = &self.config.minecraft.perks;

        let mut set = HashSet::new();
        let extra_perks = config
            .others
            .get(&UuidOrUsername::Username(view.username.clone()))
            .or_else(|| config.others.get(&UuidOrUsername::Uuid(view.uuid)));

        if let Some(extra_perks) = extra_perks {
            set.extend(extra_perks);
        }

        if view.is_contributor {
            set.extend(&config.contributors);
        }

        set.into_iter().cloned().collect::<Vec<_>>()
    }

    #[tracing::instrument(skip_all, fields(?job))]
    pub async fn enqueue_job<J: BackgroundJob + fmt::Debug>(
        &self,
        job: J,
    ) -> Result<Option<Uuid>, Report<EnqueueJobError>> {
        tracing::debug!("enqueuing job");

        let mut conn = self
            .pools
            .db_write()
            .await
            .change_context(EnqueueJobError::Database)?;

        let id = job.enqueue(&mut conn).await?;
        conn.commit()
            .await
            .change_context(EnqueueJobError::Database)?;

        Ok(id)
    }
}

impl<S: kernel_builder::State> KernelBuilder<S> {
    /// Initializes `primary_db` and `replica_db` fields lazily from the
    /// provided [database configuration].
    ///
    /// [database configuration]: eden_config::sections::Database
    pub fn pools_from_config(
        self,
        config: &eden_config::sections::Database,
    ) -> Result<KernelBuilder<kernel_builder::SetPools<S>>, Report<PoolBuildError>>
    where
        S::Pools: kernel_builder::IsUnset,
    {
        let primary_db = pool_from_config(&config.primary)
            .attach("while trying to create primary database pool")?;

        let replica_db = config
            .replica
            .as_ref()
            .map(pool_from_config)
            .transpose()
            .attach("while trying to create replica database pool")?;

        let pools = DatabasePools::builder()
            .primary_db(primary_db)
            .maybe_replica_db(replica_db)
            .build();

        Ok(self.pools(pools))
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
