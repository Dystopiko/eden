use eden_kernel::Kernel;
use error_stack::{Report, ResultExt};
use futures::StreamExt;
use splinter::{ShardConfig, ShardEventStream, ShardHandle, ShardManager};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::MissedTickBehavior;
use tokio_util::task::TaskTracker;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::queue::InMemoryQueue;
use twilight_standby::Standby;

use crate::{constants::EVENT_TYPE_FLAGS, event::EventContext};

mod constants;
mod event;

#[derive(Debug, Error)]
pub enum BotServiceError {
    #[error("Failed to start Discord bot service")]
    Start,
    #[error("A fatal error occurred in Discord bot service")]
    Fatal,
}

/// Builds [`InMemoryCache`] based on the requirements from this crate,
/// responsible handling Discord bot service inside the primary guild
/// and outside.
#[must_use]
pub fn default_in_memory_cache() -> InMemoryCache {
    let resource_types = ResourceType::GUILD
        .union(ResourceType::CHANNEL)
        .union(ResourceType::MEMBER)
        .union(ResourceType::ROLE);

    InMemoryCache::builder()
        .resource_types(resource_types)
        .build()
}

/// Starts the Discord bot service and runs until a shutdown signal is received
/// or a fatal shard error occurs in any of the running shards.
#[tracing::instrument(skip_all, name = "bot.start", level = "debug")]
pub async fn service(kernel: Arc<Kernel>) -> Result<(), Report<BotServiceError>> {
    let mut config = ShardConfig::new(
        kernel.config.bot.token.as_str().to_string(),
        self::constants::INTENTS,
        InMemoryQueue::default(),
    );
    config.event_type_flags = EVENT_TYPE_FLAGS;

    let (shard_manager, events) = ShardManager::new(config, self::constants::SHARDING_RANGE);
    let tasks = TaskTracker::new();

    tracing::debug!("spawning {} shard(s)", shard_manager.total());
    shard_manager.spawn_all().await;

    let outcome = kernel
        .shutdown_signal
        .run_result_or_cancelled(wait_until_all_identified(&shard_manager))
        .await;

    if outcome.is_err() {
        // If there's a fatal error, we may want to take down Eden IMMEDIATELY
        tracing::warn!("encountered a fatal shard error; initiating shutdown");
        kernel.shutdown_signal.initiate();
    }

    let identified = outcome?.is_some();
    let result = if identified {
        tracing::info!("discord bot service started successfully");
        tokio::spawn(dispatch_events(kernel.clone(), tasks.clone(), events));
        supervise_shards(&kernel, &shard_manager).await
    } else {
        Ok(())
    };

    // Graceful shutdown: stop accepting new tasks, drain in-flight event
    // handlers, then close every shard connection.
    tracing::debug!("closing {} shard(s)...", shard_manager.total());
    tasks.close();

    let remaining = tasks.len();
    if remaining > 0 {
        tracing::warn!("waiting for {remaining} event(s) to be processed");
        tasks.wait().await;
    }

    shard_manager.shutdown_all().await;
    tracing::info!("successfully closed {} shard(s)", shard_manager.total());

    result
}

async fn supervise_shards(
    kernel: &Arc<Kernel>,
    shard_manager: &ShardManager,
) -> Result<(), Report<BotServiceError>> {
    let mut interval = tokio::time::interval(self::constants::SUPERVISOR_CHECK_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // Consume the first, immediate tick so the first real check
    // happens after one full interval has elapsed.
    interval.tick().await;

    loop {
        // Block until the next health-check tick, or until shutdown fires.
        // `run_or_cancelled` returns `None` when the shutdown signal arrives.
        let cancelled = kernel
            .shutdown_signal
            .run_or_cancelled(interval.tick())
            .await
            .is_none();

        if cancelled {
            break Ok(());
        }

        tracing::trace!("performing periodic shard health check");
        let Some(unhealthy) = collect_unhealthy_shards(shard_manager).await else {
            continue;
        };

        // Reconnects disconnected shards concurrently
        let futures: Vec<_> = unhealthy
            .iter()
            .map(|shard| async move { (shard.id(), shard.identified().await) })
            .collect();

        for (id, result) in futures::future::join_all(futures).await {
            if let Err(error) = result {
                tracing::warn!(
                    ?error,
                    "shard {id} encountered a fatal error; initiating shutdown"
                );
                kernel.shutdown_signal.initiate();
                return Err(Report::new(error).change_context(BotServiceError::Fatal));
            }
            tracing::debug!("shard {id} recovered and is healthy again");
        }

        tracing::trace!("{} shard(s) are healthy", unhealthy.len());
    }
}

/// Forwards incoming gateway events to per-event handler tasks.
async fn dispatch_events(kernel: Arc<Kernel>, tasks: TaskTracker, mut stream: ShardEventStream) {
    tracing::debug!("event dispatcher started");

    let standby = Arc::new(Standby::new());
    while let Some((shard, event)) = stream.next().await {
        standby.process(&event);

        let ctx = EventContext::builder()
            .kernel(kernel.clone())
            .shard(shard)
            .standby(standby.clone())
            .build();

        tasks.spawn(self::event::handle(ctx, event));
    }

    tracing::debug!("event dispatcher stopped (stream exhausted)");
}

/// Returns shards that are not healthy, or `None` if every shard is healthy.
async fn collect_unhealthy_shards(shard_manager: &ShardManager) -> Option<Vec<ShardHandle>> {
    let shards = shard_manager.shards().await;
    tracing::trace!("checking health of {} shard(s)", shards.len());

    let disconnected: Vec<_> = shards
        .into_iter()
        .filter(|s| !s.state().is_identified())
        .collect();

    if disconnected.is_empty() {
        tracing::trace!("all shard(s) are healthy");
        return None;
    }

    tracing::debug!("{} shard(s) disconnected", disconnected.len());
    Some(disconnected)
}

async fn wait_until_all_identified(
    shard_manager: &ShardManager,
) -> Result<(), Report<BotServiceError>> {
    let shards = shard_manager.shards().await;
    let futures: Vec<_> = shards.iter().map(|s| s.identified()).collect();

    futures::future::try_join_all(futures)
        .await
        .change_context(BotServiceError::Start)
        .map(|_| ())
}
