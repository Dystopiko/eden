use eden_config::sections::bot::SwearingPolice as LocalConfig;
use eden_text_handling::{space_out_by_letter, swearing::RustrictType};
use eden_twilight::{PERMISSIONS_TO_SEND, http::ResponseFutureExt};
use erased_report::ErasedReport;
use rand::seq::IndexedRandom;
use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
    time::Instant,
};
use tokio::task::spawn_blocking;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{
    event::EventContext,
    triggers::{EventTrigger, EventTriggerResult},
};

pub struct SwearingPolice;

static WARNING_TEMPLATES: OnceLock<Vec<String>> = OnceLock::new();

const DEFAULT_TEMPLATES: &[&str] = &[
    "Did your mom tell you not to say {BAD_WORDS} to everyone? If you have nothing nice to say in this server, then shut up!",
    "You said {BAD_WORDS}. My goodness, you're a bad person {PREFERRED_USER_NAME}!",
    "Did you know that saying {BAD_WORDS} is/are not nice?",
    "> *Do not let any unwholesome talk come out of your mouths, but only what is helpful for building others up according to their needs, that it may benefit those who listen.*\n> \n> Ephesians 4:29 (NIV). God is watching you. So am I...",
    "Can you say something nice next time? Thank you for your cooperation! :)",
    "Your message will be reported to the server administrators. Do not ever swear again!",
    "Try to say {BAD_WORDS} again for me, please? I dare you!",
    // copied from dad bot
    "Listen here {PREFERRED_USER_NAME}, I will not tolerate you saying the words that consist of the letters {BAD_WORDS} being said in this server, so take your own advice and close thine mouth in the name of the christian minecraft server owner.",
    // New warning messages
    "Awwww, {PREFERRED_USER_NAME} said {BAD_WORDS}! This is soo cuteeee!!",
    "Wow! You said {BAD_WORDS}! Congratulations! Out of all the words in English, these are the words you went with? Absolutely remarkable!",
    "Thank you for your submission, {PREFERRED_USER_NAME}, but we regret to inform that your submission has been denied. Please allow yourself to reflect on your decisions within 3-5 days.",
    "A lot of people told me to keep quiet but I will keep quiet if you behave!",
];

impl EventTrigger for SwearingPolice {
    async fn on_message_create(
        ctx: &EventContext,
        message: &MessageCreate,
    ) -> Result<EventTriggerResult, ErasedReport> {
        let Some(guild_id) = message.guild_id else {
            return Ok(EventTriggerResult::Next);
        };

        let is_excluded = ctx
            .kernel
            .config
            .bot
            .swearing_police
            .excluded_users
            .contains(&message.author.id);

        if is_excluded {
            return Ok(EventTriggerResult::Next);
        }

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

        let Some(template) = Self::choose_warning_template(ctx.kernel.config.clone()).await else {
            return Ok(EventTriggerResult::Next);
        };

        // In due to respect to that person who's swearing, let's keep it secret :3
        let elapsed = now.elapsed();
        tracing::debug!(bad_words = ?bad_words.len(), ?elapsed, "caught someone swearing in guild!");

        let permissions = ctx.kernel.calculate_channel_permissions(
            guild_id,
            ctx.application_id.load().cast(),
            message.channel_id,
        );

        if !permissions.contains(PERMISSIONS_TO_SEND) {
            tracing::trace!("bot has no permissions to send a message");
            return Ok(EventTriggerResult::Next);
        }

        let preferred_username = message
            .member
            .as_ref()
            .and_then(|v| v.nick.as_deref())
            .or(message.author.global_name.as_deref())
            .unwrap_or_else(|| &message.author.name);

        let content = Self::format_warning_message(template, &bad_words, preferred_username);
        ctx.http
            .create_message(message.channel_id)
            .reply(message.id)
            .content(&content)
            .perform()
            .await?;

        Ok(EventTriggerResult::Next)
    }
}

impl SwearingPolice {
    async fn choose_warning_template(config: Arc<eden_config::Config>) -> Option<&'static str> {
        let result = spawn_blocking(move || {
            let mut rng = rand::rng();
            Self::get_warning_templates(&config.bot.swearing_police).choose(&mut rng)
        })
        .await;

        match result {
            Ok(okay) => okay.map(|v| &**v),
            Err(error) => {
                tracing::warn!(?error, "warning message randomizer got panicked");
                None
            }
        }
    }

    fn format_warning_message<'s>(
        template: &'s str,
        bad_words: &[String],
        preferred_username: &str,
    ) -> Cow<'s, str> {
        let mut output = Cow::Borrowed(template);

        const PREFERRED_USER_NAME_MARKER: &str = "{PREFERRED_USER_NAME}";
        if output.contains(PREFERRED_USER_NAME_MARKER) {
            output = output
                .replace(PREFERRED_USER_NAME_MARKER, preferred_username)
                .into();
        }

        const BAD_WORDS_MARKER: &str = "{BAD_WORDS}";
        if output.contains(BAD_WORDS_MARKER) {
            // Space out by every letter
            //
            // e.g.: `foo` -> `f o o`
            let mut bad_words = bad_words.iter().map(|word| space_out_by_letter(word)).fold(
                String::from("`"),
                |mut acc, word| {
                    if acc.len() > 1 {
                        acc.push_str(", ");
                    }
                    acc.push_str(&word);
                    acc
                },
            );

            bad_words.push('`');
            output = output.replace(BAD_WORDS_MARKER, &bad_words).into();
        }

        output
    }

    fn get_warning_templates(config: &LocalConfig) -> &'static [String] {
        WARNING_TEMPLATES.get_or_init(|| {
            let total_length = DEFAULT_TEMPLATES.len() + config.warning_templates.len();

            let mut templates = Vec::with_capacity(total_length);
            templates.extend(DEFAULT_TEMPLATES.iter().map(|v| v.to_string()));
            templates.extend(config.warning_templates.iter().map(|v| v.to_string()));
            templates
        })
    }
}
