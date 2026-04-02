use eden_database::{
    Settings, Timestamp,
    primary_guild::{Member, contributor::Contributor},
};
use eden_twilight::http::HttpResultExt;
use erased_report::ErasedReport;
use twilight_model::{gateway::payload::incoming::GuildCreate, guild::Guild};

use crate::{event::EventContext, primary_guild::send_welcome_message};

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

    if ctx.kernel.config.bot.primary_guild.id == guild.id {
        if let Err(error) = setup_primary_guild(&ctx, guild).await {
            tracing::warn!(?error, "failed to setup primary guild");
        }
        return;
    }
}

async fn setup_primary_guild(ctx: &EventContext, guild: &Guild) -> Result<(), ErasedReport> {
    // Check if we actually need to send welcome message
    let mut conn = ctx.kernel.pools.db_write().await?;
    let initial = &ctx.kernel.config.setup.settings;
    let (_, had_initialized) = Settings::find_or_insert(&mut conn, guild.id, initial).await?;

    if had_initialized {
        return Ok(());
    }

    tracing::debug!("settings for primary guild is not initialized; setting up local guild...");

    let user_id = ctx.application_id.load().cast();
    if let Some(member) = guild.members.iter().find(|v| v.user.id == user_id)
        && let Err(error) = send_welcome_message(&ctx.http, guild, member).await
    {
        tracing::warn!(
            ?error,
            "error occurred while trying to send welcome message to the primary guild"
        );
    }

    // Do not auto-setup contributors and anything member-related if the primary reaches
    // more than 1,000 members which is the limit for the Get Guild members endpoint in one request.
    let total_members = guild.member_count.unwrap_or(1);
    if total_members < 1000 {
        let primary_guild_cfg = &ctx.kernel.config.bot.primary_guild;
        let request = ctx.http.guild_members(primary_guild_cfg.id).limit(1000);

        let response = request.await.simplify_error()?.models();
        let members = response.await.simplify_error()?;

        tracing::debug!("looking up {} member(s)", members.len());
        for member in members {
            let is_contributor = member
                .roles
                .iter()
                .find(|&&v| v == primary_guild_cfg.contributor_role_id)
                .is_some();

            let is_member = member
                .roles
                .iter()
                .find(|&&v| v == primary_guild_cfg.member_role_id)
                .is_some()
                || is_contributor;

            if is_member {
                tracing::trace!("setting up member info for {}", member.user.id);

                let joined_at = member
                    .joined_at
                    .map(|v| Timestamp::from_secs(v.as_secs()).unwrap());

                Member::upsert()
                    .discord_user_id(member.user.id)
                    .maybe_joined_at(joined_at)
                    .name(&member.user.name)
                    .build()
                    .perform(&mut conn)
                    .await?;
            }

            if is_contributor {
                tracing::trace!("setting up contributor info for {}", member.user.id);

                Contributor::upsert()
                    .member_id(member.user.id)
                    .build()
                    .perform(&mut conn)
                    .await?;
            }
        }
    } else {
        tracing::warn!(
            "The configured primary guild reaches more than 1,000 members! It’s recommended to manually \
            set up the contributors and members, or use the Eden Admin CLI to handle the process."
        );
    }

    conn.commit().await.map_err(ErasedReport::new)?;
    Ok(())
}
