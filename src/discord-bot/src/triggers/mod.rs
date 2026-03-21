use eden_config::Config;
use error_stack::Report;
use std::sync::OnceLock;
use thiserror::Error;
use twilight_model::gateway::payload::incoming::MessageCreate;

pub mod registry;
pub mod swearing_police;

pub use self::registry::EventTriggerRegistry;

use self::swearing_police::SwearingPolice;
use crate::event::EventContext;

static LOADED_TRIGGER_REGISTRY: OnceLock<EventTriggerRegistry> = OnceLock::new();

#[allow(unused)]
pub fn init_registry(config: &Config) -> &EventTriggerRegistry {
    LOADED_TRIGGER_REGISTRY.get_or_init(|| EventTriggerRegistry::new().register::<SwearingPolice>())
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
    ) -> impl Future<Output = Result<EventTriggerResult, Report<TriggerError>>> + Send {
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
