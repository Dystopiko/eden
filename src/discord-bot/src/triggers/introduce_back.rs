use eden_text_handling::{is_maybe_word, markdown::strip_markdown, swearing::censor};
use eden_twilight::{PERMISSIONS_TO_SEND, http::ResponseFutureExt};
use erased_report::ErasedReport;
use regex::Regex;
use std::{sync::LazyLock, time::Instant};
use tokio::task::spawn_blocking;
use twilight_mention::Mention;
use twilight_model::{gateway::payload::incoming::MessageCreate, id::marker::UserMarker};

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult},
};

// I am... My name is...
static I_AM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^[Ii]|([^\w][iI])|([\s][iI]))(('?[mM])|( [aA][mM]))").unwrap());

static MY_NAME_IS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^([mM][yY])|([^\w]([mM][yY]))|([\s]([mM][yY]))) ?name(('[sS])|( ?[iI][sS]))?")
        .unwrap()
});

pub struct IntroduceBack;

impl EventTrigger for IntroduceBack {
    async fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> Result<EventTriggerResult, ErasedReport> {
        let Some(guild_id) = message.guild_id else {
            return Ok(EventTriggerResult::Next);
        };

        let now = Instant::now();
        let Some(name) = Self::try_find_name(&message.content).await else {
            return Ok(EventTriggerResult::Next);
        };

        if !is_maybe_word(&name) {
            return Ok(EventTriggerResult::Next);
        }

        let elapsed = now.elapsed();
        tracing::trace!(
            ?elapsed,
            "someone is introducing themself, introducing back to the user"
        );

        let permissions = ctx.kernel.calculate_channel_permissions(
            guild_id,
            ctx.application_id.load().cast(),
            message.channel_id,
        );

        if !permissions.contains(PERMISSIONS_TO_SEND) {
            tracing::trace!("bot has no permissions to send a message");
            return Ok(EventTriggerResult::Next);
        }

        // We only limit up to 1500 characters
        let original_size = name.len();
        let limit = original_size.clamp(1, 1500);

        let mut name = censor(&name, |c| c);
        if name.len() != limit {
            name.to_mut().push_str("...");
        }

        let content = format!(
            "Hi **{name}**, I'm {}!",
            ctx.application_id.load().cast::<UserMarker>().mention()
        );

        ctx.http
            .create_message(message.channel_id)
            .reply(message.id)
            .content(&content)
            .perform()
            .await?;

        Ok(EventTriggerResult::Stop)
    }
}

impl IntroduceBack {
    async fn try_find_name(content: &str) -> Option<String> {
        let content = content.to_string();
        let result = spawn_blocking(move || {
            let index = I_AM
                .find(&content)
                .or_else(|| MY_NAME_IS.find(&content))?
                .end();

            let name = strip_markdown(content[index..].trim_start());
            Some(name)
        })
        .await;

        match result {
            Ok(okay) => okay,
            Err(error) => {
                tracing::warn!(?error, "try_find_name got panicked");
                None
            }
        }
    }
}
