use eden_config::Config;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use twilight_gateway::Event;
use twilight_model::gateway::payload::incoming::Ready;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

mod context;

pub use self::context::EventContext;

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
        Event::Ready(inner) => handle_ready(&ctx, inner),
        _ => {}
    };
}

fn handle_ready(ctx: &EventContext, ready: Ready) {
    /// Suppresses repeated warnings when the configured application ID does not
    /// match the one Discord reports.
    static WARNED_MISMATCH: AtomicBool = AtomicBool::new(false);

    /// The application ID that ships with the bundled default configuration.
    /// Used to detect whether the operator left the placeholder value in place,
    /// in which case we skip the mismatch warning.
    static DEFAULT_APPLICATION_ID: LazyLock<Id<ApplicationMarker>> =
        LazyLock::new(|| Config::no_clone_default().bot.application_id);

    tracing::debug!(
        application.id = %ready.application.id,
        guilds = ready.guilds.len(),
        user.id = %ready.user.id,
        user.name = %ready.user.name,
        "successfully identified"
    );

    let configured_id = ctx.config().bot.application_id;
    let actual_id = ready.application.id;

    // Warn once if the operator configured a wrong application ID. The warning
    // is suppressed when they left the default placeholder (i.e. they have not
    // set it yet), since that is expected and unactionable at this stage.
    if configured_id != actual_id && configured_id != *DEFAULT_APPLICATION_ID {
        let already_warned = WARNED_MISMATCH.swap(true, Ordering::Relaxed);
        if !already_warned {
            tracing::warn!(
                configured = %configured_id,
                actual = %actual_id,
                "application ID mismatch — please update your config"
            );
        }
    }
}
