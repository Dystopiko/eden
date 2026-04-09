use std::collections::HashSet;

use constant_time_eq::constant_time_eq;
use eden_toml::TomlDiagnostic;
use error_stack::Report;
use serde::Deserialize;
use twilight_model::id::{
    Id,
    marker::{ApplicationMarker, UserMarker},
};

pub mod primary_guild;
pub use self::primary_guild::PrimaryGuild;

use crate::validate::{Validate, ValidationContext};

/// Configuration for the Discord bot.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Bot {
    #[serde(default = "default_application_id")]
    pub application_id: Id<ApplicationMarker>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub primary_guild: PrimaryGuild,
    #[serde(default)]
    pub swearing_police: SwearingPolice,
    pub token: Token,
}

#[must_use]
pub const fn default_application_id() -> Id<ApplicationMarker> {
    Id::new_checked(1).expect("one should be a valid Discord ID")
}

#[must_use]
const fn default_enabled() -> bool {
    true
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

        self.swearing_police.validate(ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct SwearingPolice {
    /// A list of user IDs that are excluded from receiving
    /// warnings from the swearing police.
    pub excluded_users: HashSet<Id<UserMarker>>,

    /// Additional list of warnings message templates for the
    /// swearing police to pick aside from the default ones.
    pub warning_templates: Vec<String>,
}

impl Default for SwearingPolice {
    fn default() -> Self {
        Self {
            excluded_users: HashSet::new(),
            warning_templates: Vec::new(),
        }
    }
}

impl Validate for SwearingPolice {
    fn validate(&self, _ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>> {
        // excluded_users unique entry validation is already checked with
        // the help of the Deserialize trait implementation for HashSet
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
