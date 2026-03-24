use eden_database::primary_guild::{McAccount, McAccountChallenge, McAccountType, Member};
use eden_sqlite::error::QueryResultExt;
use eden_twilight::http::ResponseFutureExt;
use erased_report::ErasedReport;
use sha2::Digest;
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
        if message.guild_id.is_some() {
            return Ok(EventTriggerResult::Next);
        }

        // Hash the message and try to compare it from the database
        // but make sure we're looking for a code that has no spaces inside.
        if message.content.contains(" ") {
            return Ok(EventTriggerResult::Next);
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&message.content);

        let maybe_hashed_code = hex::encode(hasher.finalize());

        let mut conn = ctx.kernel.pools.db_write().await?;
        let challenge = McAccountChallenge::find_by_hashed_code(&mut conn, &maybe_hashed_code)
            .await
            .optional()?;

        let Some(challenge) = challenge else {
            return Ok(EventTriggerResult::Next);
        };

        // Check if this user is a member of the primary guild
        let is_registered = Member::find_by_discord_user_id(&mut conn, message.author.id.cast())
            .await
            .optional()?
            .is_some();

        if !is_registered {
            ctx.http
                .create_message(message.channel_id)
                .reply(message.id)
                .content(
                    "Only verified Dystopia members are allowed to link their Minecraft account. \
                Please check the Dystopia server for how to be a member.",
                )
                .perform()
                .await?;

            McAccountChallenge::mark_cancelled(&mut conn, challenge.id).await?;
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
