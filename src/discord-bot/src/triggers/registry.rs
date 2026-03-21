use error_stack::Report;
use std::{pin::Pin, sync::Arc};
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult, TriggerError},
};

#[must_use]
pub struct EventTriggerRegistry {
    triggers: Vec<Arc<TriggerRegistryItem>>,
}

pub struct TriggerRegistryItem {
    pub name: &'static str,
    pub on_message_create: Box<TriggerCallback<MessageCreate>>,
}

impl EventTriggerRegistry {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    pub fn register<T: EventTrigger + 'static>(mut self) -> Self {
        let item: Arc<TriggerRegistryItem> = Arc::new(TriggerRegistryItem {
            name: std::any::type_name::<T>(),
            on_message_create: Box::new(on_message_create::<T>),
        });
        self.triggers.push(item);
        self
    }

    pub fn triggers(&self) -> &[Arc<TriggerRegistryItem>] {
        &self.triggers
    }
}

fn on_message_create<'a, T: EventTrigger + 'static>(
    ctx: &'a EventContext,
    message: &'a MessageCreate,
) -> TriggerFuture<'a> {
    Box::pin(T::on_message_create(ctx, message))
}

type TriggerResult = Result<EventTriggerResult, Report<TriggerError>>;
type TriggerFuture<'a> = Pin<Box<dyn Future<Output = TriggerResult> + Send + 'a>>;
type TriggerCallback<T> =
    dyn for<'a> Fn(&'a EventContext, &'a T) -> TriggerFuture<'a> + Send + Sync;
