use eden_background_worker::BackgroundJob;
use eden_database::primary_guild::logged_in_event::NewLoggedInEvent;
use eden_twilight::http::ResponseFutureExt;
use eden_utils::minecraft::{HeadIconSource, get_head_icon_url};
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, ImageSource,
};

use crate::jobs::JobContext;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct OnPlayerJoined(pub NewLoggedInEvent);

impl BackgroundJob for OnPlayerJoined {
    const TYPE: &'static str = "eden::events::player_joined";
    const TIMEOUT: Duration = Duration::from_secs(30);

    type Context = Arc<JobContext>;

    #[tracing::instrument(skip_all)]
    async fn run(&self, ctx: Self::Context) -> Result<(), ErasedReport> {
        let event = &self.0;

        let mut conn = ctx.kernel.pools.db_write().await?;
        event.create(&mut conn).await?;
        conn.commit().await.map_err(ErasedReport::new)?;

        // We only care sending alerts about the guest players, not the registered ones.
        if self.0.member_id.is_some() {
            return Ok(());
        }

        let Some(channel_id) = ctx.kernel.config.bot.primary_guild.alert_channel_id else {
            return Ok(());
        };

        if !ctx.kernel.can_send_alerts_to_discord(channel_id, None) {
            return Ok(());
        }

        let embed = generate_alert_embed(event);
        ctx.discord
            .create_message(channel_id)
            .content("**A guest player joined the server!**")
            .embeds(&[embed])
            .perform()
            .await?;

        Ok(())
    }
}

fn generate_alert_embed(event: &NewLoggedInEvent) -> Embed {
    let icon_url = ImageSource::url(get_head_icon_url(HeadIconSource::Uuid(event.player_uuid)))
        .expect("get_head_icon_url should produce valid URL");

    let author_name = event.username.as_deref().unwrap_or("Guest");
    let author_field = EmbedAuthorBuilder::new(author_name)
        .icon_url(icon_url)
        .build();

    let event_id = EmbedFieldBuilder::new("Event ID", format!("`{}`", event.event_id)).inline();
    let ip_addr = EmbedFieldBuilder::new("IP Address", format!("`{}`", event.ip_address)).inline();
    let account_type = EmbedFieldBuilder::new("Account Type", format!("`{}`", event.kind)).inline();

    EmbedBuilder::new()
        .author(author_field)
        .field(event_id)
        .field(ip_addr)
        .field(account_type)
        .timestamp(event.created_at.into_twilight())
        .build()
}
