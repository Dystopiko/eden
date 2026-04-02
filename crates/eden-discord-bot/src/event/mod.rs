use bon::Builder;
use crossbeam::atomic::AtomicCell;
use eden_core::Kernel;
use splinter::ShardHandle;
use std::sync::Arc;
use twilight_gateway::Event;
use twilight_model::id::{Id, marker::ApplicationMarker};
use twilight_standby::Standby;

mod guild_create;
mod message_create;
mod ready;

#[derive(Debug, Builder)]
#[allow(unused)]
pub struct EventContext {
    pub application_id: Arc<AtomicCell<Id<ApplicationMarker>>>,
    pub kernel: Arc<Kernel>,
    pub http: Arc<twilight_http::Client>,
    pub shard: ShardHandle,

    /// Retained for future use by triggers/commands that need to wait for a
    /// specific follow-up event (e.g. a button interaction or a message reply).
    #[allow(dead_code)]
    pub standby: Arc<Standby>,
}

#[allow(clippy::single_match, clippy::match_single_binding)]
#[tracing::instrument(
    skip_all,
    name = "bot.handle_event",
    fields(
        event.kind = ?event.kind(),
        shard.id = %ctx.shard.id(),
        shard.latency = ?ctx.shard.latency(),
    ),
)]
pub async fn handle(ctx: EventContext, event: Event) {
    tracing::trace!("received event");

    match event {
        Event::GuildCreate(guild) => self::guild_create::handle(ctx, &guild).await,
        Event::MessageCreate(inner) => self::message_create::handle(&ctx, inner).await,
        Event::Ready(ready) => self::ready::handle(&ctx, &ready),
        _ => {}
    };
}
