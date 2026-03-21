use eden_database::Settings;
use erased_report::ErasedReport;
use twilight_model::{
    gateway::payload::incoming::GuildCreate,
    guild::{Guild, Member},
};

use crate::event::EventContext;

#[tracing::instrument(
    skip_all,
    name = "bot.events.guild_create",
    fields(
        guild.id = ?event.id(),
        guild.unavailable = matches!(event, GuildCreate::Unavailable(..))
    ),
)]
pub async fn handle(ctx: EventContext, event: &GuildCreate) {
    tracing::trace!("received guild create event");

    let GuildCreate::Available(guild) = event else {
        return;
    };

    let user_id = ctx.application_id.load().cast();
    let Some(member) = guild.members.iter().find(|v| v.user.id == user_id) else {
        return;
    };

    if ctx.kernel.config.bot.primary_guild.id != guild.id {
        return;
    }

    if let Err(error) = try_send_welcome_message(&ctx, guild, member).await {
        tracing::warn!(
            ?error,
            "error occurred while trying to send welcome message to the primary guild"
        );
    }
}

async fn try_send_welcome_message(
    ctx: &EventContext,
    guild: &Guild,
    member: &Member,
) -> Result<(), ErasedReport> {
    // Check if we actually need to send welcome message
    let mut conn = ctx.kernel.pools.db_write().await?;

    let initial = &ctx.kernel.config.setup.settings;
    let (_, had_initialized) = Settings::find_or_insert(&mut conn, guild.id, initial).await?;

    if had_initialized {
        return Ok(());
    }

    tracing::debug!("settings for primary guild is not initialized; sending welcome message...");
    crate::primary_guild::send_welcome_message(&ctx.http, guild, member).await?;
    conn.commit().await.map_err(ErasedReport::new)?;

    Ok(())
}
