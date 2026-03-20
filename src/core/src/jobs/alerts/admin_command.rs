use eden_background_worker::BackgroundJob;
use eden_database::Timestamp;
use eden_gateway_api::alerts::admin_commands::{
    AdminCommandAlert as EncodedAdminCommandAlert, Executor, ExecutorPlayerInfo,
};
use eden_text_handling::markdown::strip_markdown;
use eden_twilight::http::ResponseFutureExt;
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc, time::Duration};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, ImageSource,
};

use crate::jobs::JobContext;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AdminCommandAlertJob(pub EncodedAdminCommandAlert);

impl BackgroundJob for AdminCommandAlertJob {
    const TYPE: &'static str = "eden::alerts::admin_command";
    const TIMEOUT: Duration = Duration::from_mins(1);

    type Context = Arc<JobContext>;

    #[tracing::instrument(skip_all, fields(
        alert.command = ?self.0.command,
        alert.executor = ?self.0.executor,
    ))]
    async fn run(&self, ctx: Self::Context) -> Result<(), ErasedReport> {
        let alert = &self.0;
        let Some(alert_channel_id) = ctx.kernel.config.bot.primary_guild.alert_channel_id else {
            return Ok(());
        };

        let partial_embed = EmbedBuilder::new()
            .title(format!("`{}`", strip_markdown(&alert.command)))
            .timestamp(Timestamp::now().into_twilight());

        let (content, embed) = match &alert.executor {
            Executor::Console => {
                let content = Cow::Borrowed("**Someone used a privileged command!!**");
                let embed = partial_embed.author(EmbedAuthorBuilder::new("Console").build());
                (content, embed)
            }
            Executor::Player(info) => {
                let content = format!("**{} used a privileged command!!**", info.username);
                let embed = embed_for_player_executor(partial_embed, info);
                (Cow::Owned(content), embed)
            }
        };

        let embed = embed.build();
        ctx.discord
            .create_message(alert_channel_id)
            .content(&content)
            .embeds(&[embed])
            .perform()
            .await?;

        Ok(())
    }
}

fn embed_for_player_executor(embed: EmbedBuilder, player: &ExecutorPlayerInfo) -> EmbedBuilder {
    let icon_url = ImageSource::url(get_head_icon_url(&player.username))
        .expect("get_head_icon_url should produce valid URL");

    let author_field = EmbedAuthorBuilder::new(&player.username)
        .icon_url(icon_url)
        .build();

    let uuid_field = EmbedFieldBuilder::new("UUID", format!("`{}`", player.uuid)).inline();
    let dim_field = EmbedFieldBuilder::new("Dimension", format!("`{}`", player.dimension)).inline();
    let position_field = EmbedFieldBuilder::new(
        "Position",
        format!(
            "`{}, {}, {}`",
            player.position.x, player.position.y, player.position.z
        ),
    )
    .inline();

    let gamemode_field =
        EmbedFieldBuilder::new("Game Mode", format!("`{}`", player.gamemode)).inline();

    embed
        .author(author_field)
        .field(uuid_field)
        .field(dim_field)
        .field(position_field)
        .field(gamemode_field)
}

fn get_head_icon_url(username: &str) -> String {
    const HEAD_ICON_BASE_URL: &str = "https://minotar.net/avatar/";

    let mut url = HEAD_ICON_BASE_URL.to_string();
    url.extend(percent_encoding::percent_encode(
        username.as_bytes(),
        percent_encoding::NON_ALPHANUMERIC,
    ));
    url
}

#[cfg(test)]
mod tests {
    use super::get_head_icon_url;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_get_head_icon_url() {
        assert_eq!(
            get_head_icon_url("Notch"),
            "https://minotar.net/avatar/Notch"
        );

        // Bedrock supports usernames with spaces, so the username
        // must be percent-encoded to produce a valid URL.
        assert_eq!(
            get_head_icon_url("Ordinary Player"),
            "https://minotar.net/avatar/Ordinary%20Player"
        );
    }
}
