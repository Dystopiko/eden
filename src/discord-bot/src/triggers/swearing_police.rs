use eden_text_handling::swearing::RustrictType;
use eden_twilight::{PERMISSIONS_TO_SEND, http::ResponseFutureExt};
use erased_report::ErasedReport;
use std::time::Instant;
use tokio::task::spawn_blocking;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult},
};

pub struct SwearingPolice;

impl EventTrigger for SwearingPolice {
    async fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> Result<EventTriggerResult, ErasedReport> {
        let Some(guild_id) = message.guild_id else {
            return Ok(EventTriggerResult::Next);
        };

        let now = Instant::now();
        let bad_words = {
            // find_bad_words is a heavy function, give some time to process
            let content = message.content.to_string();
            spawn_blocking(move || {
                eden_text_handling::swearing::find_bad_words(&content, |c| {
                    c.with_censor_threshold(RustrictType::OFFENSIVE | RustrictType::PROFANE)
                })
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        };

        if bad_words.is_empty() {
            return Ok(EventTriggerResult::Next);
        }

        // In due to respect to that person who's swearing, let's keep it secret :3
        let elapsed = now.elapsed();
        tracing::debug!(bad_words = ?bad_words.len(), ?elapsed, "caught someone swearing in guild!");

        let permissions = ctx.kernel.calculate_channel_permissions(
            guild_id,
            ctx.application_id.load().cast(),
            message.channel_id,
        );

        if !permissions.contains(PERMISSIONS_TO_SEND) {
            return Ok(EventTriggerResult::Next);
        }

        ctx.http
            .create_message(message.channel_id)
            .reply(message.id)
            .content("Please stop swearing!")
            .perform()
            .await?;

        Ok(EventTriggerResult::Next)
    }
}
