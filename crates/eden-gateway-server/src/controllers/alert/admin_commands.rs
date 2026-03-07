use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_database::Timestamp;
use eden_text_handling::markdown::strip_markdown;
use eden_twilight::http::ResponseFutureExt;
use percent_encoding::NON_ALPHANUMERIC;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, ImageSource,
};
use uuid::Uuid;

use crate::{
    controllers::{ApiResult, Kernel},
    model::{McBlockPosition, McGameMode},
};

// TODO: Input validation
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PublishAlert {
    pub command: String,
    pub executor: Executor,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Executor {
    Console,
    Player {
        dimension: String,
        game_mode: Option<McGameMode>,
        position: McBlockPosition,
        username: String,
        uuid: Uuid,
    },
}

pub async fn publish(
    Kernel(kernel): Kernel,
    Json(mut body): Json<PublishAlert>,
) -> ApiResult<Response> {
    // Strip any markdown in command field because this will affect the embed soon.
    body.command = strip_markdown(&body.command);

    let embed = generate_alert_embed(&body);
    let content = match &body.executor {
        Executor::Console => Cow::Borrowed("**Someone used a privileged command!!**"),
        Executor::Player { username, .. } => {
            Cow::Owned(format!("**{username} used a privileged command!!**"))
        }
    };

    let request = kernel
        .discord
        .create_message(kernel.config.bot.primary_guild.alert_channel_id);

    request
        .content(&*content)
        .embeds(&[embed])
        .perform()
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

// https://github.com/memothelemo/albasset/blob/master/src/main/kotlin/xyz/memothelemo/albasset/alert/AdminCommandExecutedAlert.kt
fn generate_alert_embed(body: &PublishAlert) -> Embed {
    let mut embed = EmbedBuilder::new()
        .title(format!("`{}`", body.command))
        .timestamp(Timestamp::now().into_twilight());

    embed = match &body.executor {
        Executor::Console => embed.author(EmbedAuthorBuilder::new("Console").build()),
        Executor::Player {
            dimension,
            game_mode,
            position,
            username,
            uuid,
        } => {
            let icon_url = ImageSource::url(get_head_icon_url(username))
                .expect("get_head_icon_url should produce valid URL");

            let mut embed = embed
                .author(EmbedAuthorBuilder::new(username).icon_url(icon_url).build())
                .field(EmbedFieldBuilder::new("UUID", format!("`{uuid}`")).inline())
                .field(EmbedFieldBuilder::new("Dimension", format!("`{dimension}`")).inline())
                .field(
                    EmbedFieldBuilder::new(
                        "Position",
                        format!("`{}, {}, {}`", position.x, position.y, position.z),
                    )
                    .inline(),
                );

            if let Some(game_mode) = game_mode {
                let field = EmbedFieldBuilder::new("Game Mode", format!("`{game_mode}`")).inline();
                embed = embed.field(field);
            }

            embed
        }
    };

    embed.build()
}

fn get_head_icon_url(username: &str) -> String {
    const HEAD_ICON_BASE_URL: &str = "https://minotar.net/avatar/";

    let mut url = HEAD_ICON_BASE_URL.to_string();
    url.extend(percent_encoding::percent_encode(
        username.as_bytes(),
        NON_ALPHANUMERIC,
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
