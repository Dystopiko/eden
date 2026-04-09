use eden_background_worker::BackgroundJob;
use eden_core::jobs::CancelMcAccountChallenge;
use eden_database::primary_guild::{McAccount, McAccountChallenge, McAccountType, Member};
use eden_sqlite::error::QueryResultExt;
use eden_twilight::http::ResponseFutureExt;
use erased_report::{EraseReportExt, ErasedReport};
use error_stack::ResultExt;
use sha2::Digest;
use thiserror::Error;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult},
};

pub struct SolveMcAccountChallenge;

impl EventTrigger for SolveMcAccountChallenge {
    async fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> Result<EventTriggerResult, ErasedReport> {
        let primary_guild_id = ctx.kernel.config.bot.primary_guild.id;
        let should_scan_for_codes =
            message.guild_id == Some(primary_guild_id) || message.guild_id.is_none();

        if !should_scan_for_codes {
            return Ok(EventTriggerResult::Next);
        }

        // Hash the message and try to compare it from the database
        // but make sure we're looking for a code that has no spaces inside.
        if message.content.contains(" ") {
            return Ok(EventTriggerResult::Next);
        }

        let mut conn = ctx.kernel.pools.db_write().await?;

        let maybe_hashed_code = Self::hash_content(message.content.clone()).await?;
        let challenge = McAccountChallenge::find_by_hashed_code(&mut conn, &maybe_hashed_code)
            .await
            .optional()?;

        let Some(challenge) = challenge else {
            return Ok(EventTriggerResult::Next);
        };

        // If the code is sent to the primary guild, cancel the challenge
        // immediately and send it back to user.
        if message.guild_id == Some(primary_guild_id) {
            CancelMcAccountChallenge(challenge.id)
                .enqueue(&mut conn)
                .await?;

            let dm_channel_id = ctx
                .http
                .create_private_channel(message.author.id)
                .model()
                .await
                .attach("while trying to create private channel to alert the player")?
                .id;

            ctx.http
                .create_message(dm_channel_id)
                .content(
                    "Please send the verification code that Eden provided here, next time. Run `/eden link` to \
                    restart the Minecraft account linking process. (this is required for security)",
                )
                .perform()
                .await?;

            return Ok(EventTriggerResult::Stop);
        }

        // Check if this user is a member of the primary guild
        let is_registered = Member::find_by_discord_user_id(&mut conn, message.author.id.cast())
            .await
            .optional()?
            .is_some();

        if !is_registered {
            CancelMcAccountChallenge(challenge.id)
                .enqueue(&mut conn)
                .await?;

            ctx.http
                .create_message(message.channel_id)
                .reply(message.id)
                .content(
                    "Only verified Dystopia members are allowed to link their Minecraft account. \
                Please check the Dystopia server for how to be a member.",
                )
                .perform()
                .await?;

            return Ok(EventTriggerResult::Stop);
        }

        let account_type = if challenge.java {
            McAccountType::Java
        } else {
            McAccountType::Bedrock
        };

        McAccount::new()
            .account_type(account_type)
            .discord_user_id(message.author.id)
            .username(&challenge.username)
            .uuid(challenge.uuid)
            .build()
            .create(&mut conn)
            .await?;

        McAccountChallenge::mark_done(&mut conn, challenge.id).await?;
        ctx.http
            .create_message(message.channel_id)
            .reply(message.id)
            .content(&format!(
                "Successfully linked `{}` Minecraft account to your Discord account! Please \
                rejoin the Minecraft server to reflect changes.",
                challenge.username,
            ))
            .perform()
            .await?;

        conn.commit().await.map_err(ErasedReport::new)?;
        Ok(EventTriggerResult::Stop)
    }
}

impl SolveMcAccountChallenge {
    async fn hash_content(content: String) -> Result<String, ErasedReport> {
        #[derive(Debug, Error)]
        #[error("tokio task panicked while generating a hash from a message content")]
        struct TaskPanicked;

        tokio::task::spawn_blocking(move || {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&content);
            hex::encode(hasher.finalize())
        })
        .await
        .change_context(TaskPanicked)
        .erase_report()
    }
}
