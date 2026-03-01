use constant_time_eq::constant_time_eq;
use eden_toml::TomlDiagnostic;
use error_stack::Report;
use serde::Deserialize;

use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use crate::validate::{Validate, ValidationContext};

pub mod primary_guild;
pub use self::primary_guild::PrimaryGuild;

/// Configuration for the Discord bot.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Bot {
    /// The application ID of the associated Discord bot.
    pub application_id: Id<ApplicationMarker>,

    /// Primary guild/server is where all of Eden's features will take place such
    /// as payment processes, Minecraft server integration, administration, and
    /// many to add in the future.
    ///
    /// Other guilds will receive limited features offered by Eden.
    pub primary_guild: PrimaryGuild,

    /// The Discord bot token used for authentication.
    pub token: Token,
}

impl Validate for Bot {
    fn validate(&self, ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>> {
        let token = self.token.as_str();
        let has_valid_chars = token
            .chars()
            .all(|c| c.is_ascii() && !c.is_whitespace() && !c.is_control());

        if token.is_empty() || !has_valid_chars {
            let token_span = ctx
                .document
                .get("bot")
                .and_then(|section| section.get("token"))
                .and_then(|value| value.span());

            let diagnostic = eden_toml::diagnostic(
                "Got invalid Discord token",
                token_span,
                ctx.source,
                ctx.path,
            );

            return Err(diagnostic);
        }

        Ok(())
    }
}

/// A wrapper for a Discord bot authorization token allocated in
/// the heap with debug implementation that redacts the entire string.
///
/// The user is responsible for handling the token and avoiding
/// the token from being exposed in the stack memory.
#[derive(Clone, Default)]
pub struct Token {
    inner: Box<str>,
}

impl Token {
    /// Creates a new [`Token`] wrapping `value`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let inner = value.into().into_boxed_str();
        Self { inner }
    }

    /// Returns the raw token value as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl PartialEq for Token {
    /// Compares tokens in constant time to prevent timing side-channels.
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(self.inner.as_bytes(), other.inner.as_bytes())
    }
}

impl Eq for Token {}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Token").finish_non_exhaustive()
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Token;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Discord bot token string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Token::new(v))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
