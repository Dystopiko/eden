use eden_database::primary_guild::{Chaos, chaos::UpdateChaosError};
use eden_twilight::http::ResponseFutureExt;
use erased_report::ErasedReport;
use error_stack::{Report, ResultExt};
use num_format::{Locale, ToFormattedString};
use std::borrow::Cow;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult},
};

pub struct ChaosNecoEmoticon;

impl EventTrigger for ChaosNecoEmoticon {
    fn is_enabled(config: &eden_config::Config) -> bool
    where
        Self: Sized,
    {
        !config.bot.primary_guild.chaosneco_user_ids.is_empty()
    }

    #[tracing::instrument(skip_all, name = "bot.triggers.chaos")]
    async fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> Result<EventTriggerResult, ErasedReport> {
        // Only react to messages sent by PomPom inside the primary guild.
        let is_chaosneco = ctx
            .kernel
            .config
            .bot
            .primary_guild
            .chaosneco_user_ids
            .iter()
            .find(|&v| *v == message.author.id)
            .is_some();

        let in_primary_guild = Some(ctx.kernel.config.bot.primary_guild.id) == message.guild_id;
        if !is_chaosneco || !in_primary_guild {
            return Ok(EventTriggerResult::Next);
        }

        if !message.content.contains(";-;") {
            return Ok(EventTriggerResult::Next);
        }

        tracing::trace!("Pom sent a crying emoticon — mirroring it back");

        let content = match Self::increment_emoticon_times(ctx).await {
            Ok(times) => Cow::Owned(format!(
                ";-; part {}",
                times.to_formatted_string(&Locale::en_US_POSIX)
            )),
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "failed to increment times of sending crying emoticons"
                );
                eden_sentry::capture_report(&error);
                Cow::Borrowed(";-;")
            }
        };

        let result = ctx
            .http
            .create_message(message.channel_id)
            .reply(message.id)
            .content(&content)
            .perform()
            .await;

        if let Err(error) = result {
            tracing::warn!(?error, "failed to send mirror message to Pom");
        }

        Ok(EventTriggerResult::Stop)
    }
}

impl ChaosNecoEmoticon {
    async fn increment_emoticon_times(
        ctx: &EventContext,
    ) -> Result<usize, Report<UpdateChaosError>> {
        let mut conn = ctx
            .kernel
            .pools
            .db_write()
            .await
            .change_context(UpdateChaosError)?;

        let times = Chaos::add_crying_times(&mut conn)
            .await?
            .crying_emoticon_times;

        let times = if let Ok(times) = usize::try_from(times) {
            times
        } else {
            tracing::warn!(?times, "crying_times reached the limits of usize!");
            1
        };

        conn.commit().await.change_context(UpdateChaosError)?;
        Ok(times)
    }
}
