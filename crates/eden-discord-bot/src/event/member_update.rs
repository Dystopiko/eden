use erased_report::ErasedReport;
use twilight_model::gateway::payload::incoming::MemberUpdate;

use crate::event::EventContext;

#[tracing::instrument(
    skip_all,
    name = "bot.events.member_update",
    fields(
        %member.user.id,
        %member.pending,
        %member.guild_id,
    )
)]
pub async fn handle(ctx: &EventContext, member: Box<MemberUpdate>) {
    // Do not proceed if the member has yet to pass the guild's membership screening
    // requirements or maybe the user is a bot.
    if member.pending || member.user.bot {
        return;
    }

    let primary_guild_id = ctx.kernel.config.bot.primary_guild.id;
    if primary_guild_id == dbg!(member.guild_id)
        && let Err(error) = handle_for_primary_guild(ctx, &member).await
    {
        tracing::error!(
            ?error,
            "error occurred while handling member updates for primary guild"
        );
    }
}

async fn handle_for_primary_guild(
    ctx: &EventContext,
    member: &MemberUpdate,
) -> Result<(), ErasedReport> {
    let local_cfg = &ctx.kernel.config.bot.primary_guild;
    let is_member = member.roles.contains(&local_cfg.member_role_id);
    dbg!(is_member);

    if !is_member {
        tracing::trace!("{} is not a member", member.user.id);
        return Ok(());
    }

    tracing::debug!("updating member info for {}", member.user.id);

    let mut conn = ctx.kernel.pools.db_write().await?;
    crate::primary_guild::setup_member(
        &ctx.kernel,
        &mut conn,
        member.joined_at,
        &member.roles,
        &member.user,
    )
    .await?;

    conn.commit().await.map_err(ErasedReport::new)?;
    Ok(())
}
