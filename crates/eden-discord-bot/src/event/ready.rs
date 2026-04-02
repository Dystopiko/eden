use eden_config::sections::bot::default_application_id;
use std::sync::atomic::{AtomicBool, Ordering};
use twilight_model::gateway::payload::incoming::Ready;

use crate::event::EventContext;

/// Suppresses repeated warnings when the configured application ID does not
/// match the one Discord reports.
static WARNED_MISMATCH: AtomicBool = AtomicBool::new(false);

pub fn handle(ctx: &EventContext, ready: &Ready) {
    tracing::debug!(
        application.id = %ready.application.id,
        guilds = ready.guilds.len(),
        user.id = %ready.user.id,
        user.name = %ready.user.name,
        "successfully identified"
    );

    let configured_id = ctx.application_id.load();
    let actual_id = ready.application.id;
    ctx.application_id.store(actual_id);

    // Warn once if the operator configured a wrong application ID. The warning
    // is suppressed when they left the default placeholder (i.e. they have not
    // set it yet), since that is expected and unactionable at this stage.
    if configured_id != actual_id && configured_id != default_application_id() {
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
