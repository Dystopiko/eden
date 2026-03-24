use tracing::Instrument;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTriggerResult, TriggerError},
};

#[tracing::instrument(
    skip_all,
    name = "bot.events.message",
    fields(
        %message.id,
        %message.author.id,
        message.channel.id = %message.channel_id,
        message.guild.id = tracing::field::Empty,
        ?message.kind,
        ?message.timestamp,
    )
)]
pub async fn handle(ctx: &EventContext, message: Box<MessageCreate>) {
    // Ignore messages sent by other bots to avoid feedback loops.
    if message.author.bot {
        return;
    }

    if let Some(guild_id) = message.guild_id {
        tracing::Span::current().record("message.guild.id", tracing::field::display(guild_id));
    }

    tracing::trace!("received human message");

    let registry = crate::triggers::init_registry(&ctx.kernel.config);
    for trigger in registry.triggers() {
        let span = tracing::info_span!(
            "bot.triggers.run",
            trigger.name = %trigger.name,
        );

        let result = (*trigger.on_message_create)(ctx, &message)
            .instrument(span.clone())
            .await
            .map_err(|r| r.change_context(TriggerError));

        let next = match result {
            Ok(next) => next,
            Err(error) => {
                span.in_scope(|| tracing::warn!(?error, "failed to process trigger"));
                break;
            }
        };

        span.in_scope(|| tracing::trace!(result = ?next, "done processing trigger"));
        match next {
            EventTriggerResult::Next => continue,
            EventTriggerResult::Stop => break,
        }
    }
}
