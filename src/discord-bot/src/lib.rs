use crossbeam::atomic::AtomicCell;
use eden_core::Kernel;
use error_stack::{Report, ResultExt};
use futures::StreamExt;
use splinter::{ShardConfig, ShardEventStream, ShardHandle, ShardManager, ShardingRange};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::time::MissedTickBehavior;
use tokio_util::task::TaskTracker;
use twilight_gateway::{CloseFrame, EventTypeFlags, Intents, queue::InMemoryQueue};
use twilight_standby::Standby;

mod event;
mod primary_guild;
mod triggers;

use self::event::EventContext;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Failed to start Discord bot service")]
    Start,

    #[error("A fatal error occurred in Discord bot service")]
    FatallyClosed,
}

const EVENT_TYPE_FLAGS: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::GUILD_CREATE)
    .union(EventTypeFlags::MESSAGE_CREATE);

const INTENTS: Intents = Intents::DIRECT_MESSAGES
    .union(Intents::GUILDS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::MESSAGE_CONTENT);

const SUPERVISOR_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const SHARDING_RANGE: ShardingRange = ShardingRange::ONE;

pub async fn service(
    kernel: Arc<Kernel>,
    http: Arc<twilight_http::Client>,
) -> Result<(), Report<ServiceError>> {
    let mut config = ShardConfig::new(
        kernel.config.bot.token.as_str().to_string(),
        INTENTS,
        InMemoryQueue::default(),
    );
    config.event_type_flags = EVENT_TYPE_FLAGS;

    let (shard_manager, events) = ShardManager::new(config, SHARDING_RANGE);
    let tasks = TaskTracker::new();

    tracing::debug!("spawning {} shard(s)", shard_manager.total());
    shard_manager.spawn_all().await;

    let identified = kernel
        .shutdown_signal
        .run_result_or_cancelled(wait_until_all_identified(&shard_manager))
        .await?
        .is_some();

    let result = if identified {
        tracing::info!("Discord bot service started successfully");
        tokio::spawn(dispatch_events(kernel.clone(), http, tasks.clone(), events));
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

    for shard in shard_manager.shards().await {
        _ = shard.close(CloseFrame::NORMAL);
    }
    shard_manager.shutdown_all().await;
    tracing::info!("successfully closed {} shard(s)", shard_manager.total());

    result
}

async fn supervise_shards(
    kernel: &Arc<Kernel>,
    shard_manager: &ShardManager,
) -> Result<(), Report<ServiceError>> {
    let mut interval = tokio::time::interval(SUPERVISOR_CHECK_INTERVAL);
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
                return Err(Report::new(error).change_context(ServiceError::FatallyClosed));
            }
            tracing::debug!("shard {id} recovered and is healthy again");
        }

        tracing::trace!("{} shard(s) are healthy", unhealthy.len());
    }
}

/// Forwards incoming gateway events to per-event handler tasks.
async fn dispatch_events(
    kernel: Arc<Kernel>,
    http: Arc<twilight_http::Client>,
    tasks: TaskTracker,
    mut stream: ShardEventStream,
) {
    tracing::debug!("event dispatcher started");

    let application_id = Arc::new(AtomicCell::new(kernel.config.bot.application_id));
    let standby = Arc::new(Standby::new());

    while let Some((shard, event)) = stream.next().await {
        kernel.discord_cache.update(&event);
        standby.process(&event);

        let ctx = EventContext::builder()
            .application_id(application_id.clone())
            .kernel(kernel.clone())
            .http(http.clone())
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
) -> Result<(), Report<ServiceError>> {
    let shards = shard_manager.shards().await;
    let futures: Vec<_> = shards.iter().map(|s| s.identified()).collect();
    futures::future::try_join_all(futures)
        .await
        .change_context(ServiceError::Start)
        .map(|_| ())
}
