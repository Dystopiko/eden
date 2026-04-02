use eden_config::Config;
use erased_report::ErasedReport;
use std::sync::OnceLock;
use thiserror::Error;
use twilight_model::gateway::payload::incoming::MessageCreate;

pub mod chaosneco;
pub mod introduce_back;
pub mod registry;
pub mod solve_mc_account_challenge;
pub mod swearing_police;

pub use self::registry::EventTriggerRegistry;

use crate::event::EventContext;

static LOADED_TRIGGER_REGISTRY: OnceLock<EventTriggerRegistry> = OnceLock::new();

#[allow(unused)]
pub fn init_registry(config: &Config) -> &EventTriggerRegistry {
    LOADED_TRIGGER_REGISTRY.get_or_init(|| {
        EventTriggerRegistry::new()
            .register::<self::introduce_back::IntroduceBack>()
            .register::<self::chaosneco::ChaosNecoEmoticon>()
            .register::<self::swearing_police::SwearingPolice>()
            .register::<self::solve_mc_account_challenge::SolveMcAccountChallenge>()
    })
}

#[allow(unused)]
pub trait EventTrigger: Send + Sync {
    /// Returns whether this trigger is currently enabled. Disabled triggers are
    /// skipped entirely in the list of triggers.
    ///
    /// Defaults to `true`.
    #[must_use]
    fn is_enabled(config: &Config) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> impl Future<Output = Result<EventTriggerResult, ErasedReport>> + Send {
        async { Ok(EventTriggerResult::Next) }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventTriggerResult {
    /// The trigger has been performed successfully or not.
    ///
    /// Incoming triggers will be triggered.
    Next,

    /// The trigger has been performed successfully or not.
    ///
    /// Incoming triggers will not be triggered.
    Stop,
}

#[derive(Debug, Error)]
#[error("failed to process trigger")]
pub struct TriggerError;
