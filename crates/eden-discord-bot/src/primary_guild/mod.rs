use eden_twilight::{PERMISSIONS_TO_SEND, find_everyone_role, http::ResponseFutureExt};
use error_stack::{Report, ResultExt};
use std::collections::HashMap;
use thiserror::Error;
use twilight_model::{
    channel::{Channel, ChannelType},
    guild::{Guild, Member, Permissions},
};
use twilight_util::permission_calculator::PermissionCalculator;
use unindent::unindent;

/// Error returned when the welcome message could not be delivered.
#[derive(Debug, Error)]
#[error("Failed to send welcome message")]
pub struct SendWelcomeMessageError;

/// Welcome message sent to the primary guild when Eden first joins it.
#[allow(dead_code)]
pub const PRIMARY_GUILD_WELCOME_MESSAGE: &str = concat!(
    "**Thank you for choosing Eden as your primary Discord bot for your Minecraft server needs!** :laughing:

    **Please bare mind that this bot is in development phase. Bugs are expected to occur at anytime**. If you encountered bugs/issues with this bot, don't hesitate to report us at: ",
    env!("CARGO_PKG_REPOSITORY"),
    "/issues",
);

/// Sends the primary-guild welcome message on behalf of the bot.
///
/// The function attempts to find the first sendable, non-NSFW text channel in
/// `guild` that the bot has sufficient permissions (`VIEW_CHANNEL` and `SEND_MESSAGES`)
/// to send the message in (ordered by creation time). If no such channel exists
/// it falls back to opening a DM with the guild owner.
#[tracing::instrument(
    skip_all,
    fields(
        channel.id = tracing::field::Empty,
        channel.kind = tracing::field::Empty,
        guild.id = %guild.id,
        guild.owner.id = %guild.owner_id,
    ),
)]
pub async fn send_welcome_message(
    http: &twilight_http::Client,
    guild: &Guild,
    member: &Member,
) -> Result<(), Report<SendWelcomeMessageError>> {
    let guild_text_channel = find_sendable_text_channel(guild, member);

    let span = tracing::Span::current();
    let (channel_id, channel_kind) = match guild_text_channel {
        Some(channel) => (channel.id, channel.kind),
        None => {
            tracing::debug!(
                "no sendable text channel found; falling back to direct \
                message with the guild owner"
            );

            let channel = http
                .create_private_channel(guild.owner_id)
                .model()
                .await
                .change_context(SendWelcomeMessageError)
                .attach(
                    "while trying to create a DM channel for bot and the primary guild owner",
                )?;

            (channel.id, channel.kind)
        }
    };

    span.record("channel.id", tracing::field::display(channel_id));
    span.record("channel.kind", tracing::field::debug(channel_kind));

    http.create_message(channel_id)
        .content(&unindent(PRIMARY_GUILD_WELCOME_MESSAGE))
        .model()
        .await
        .change_context(SendWelcomeMessageError)?;

    tracing::info!("sent welcome message");
    Ok(())
}

#[must_use = "returns the sendable channels; ignoring it means the lookup was pointless"]
fn find_sendable_text_channel<'g>(guild: &'g Guild, member: &Member) -> Option<&'g Channel> {
    // Role resolution is O(len(member_roles) + len(guild_roles)) if hash maps are used.
    let role_perms_map = guild
        .roles
        .iter()
        .map(|r| (r.id, r.permissions))
        .collect::<HashMap<_, _>>();

    let member_roles = member
        .roles
        .iter()
        .filter_map(|id| role_perms_map.get(id).map(|v| (*id, *v)))
        .collect::<Vec<_>>();

    let everyone = find_everyone_role(&guild.roles)
        .map(|r| r.permissions)
        .unwrap_or_else(Permissions::empty);

    let calculator = PermissionCalculator::new(guild.id, member.user.id, everyone, &member_roles);

    let mut text_channels = guild
        .channels
        .iter()
        .filter(|c| c.kind == ChannelType::GuildText && !c.nsfw.unwrap_or(false));

    text_channels.find(|c| {
        let overwrites = c.permission_overwrites.as_deref().unwrap_or_default();
        let perms = calculator.clone().in_channel(c.kind, overwrites);
        perms.contains(PERMISSIONS_TO_SEND)
    })
}
