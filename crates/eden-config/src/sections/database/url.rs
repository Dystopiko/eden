use constant_time_eq::constant_time_eq;
use eden_utils::sensitive::Sensitive;
use serde::Deserialize;
use std::fmt;

/// An SQLite connection URL, either file-backed or in-memory.
#[derive(Debug, Clone)]
pub enum SqliteUrl {
    /// A real file-backed or URI SQLite connection string.
    Url(Sensitive<String>),

    /// An in-memory SQLite database.
    ///
    /// The inner value is the raw connection string, if it contained query
    /// parameters or credentials. `None` for plain in-memory URLs with no extras.
    Memory(Option<String>),
}

impl SqliteUrl {
    /// Leaks the redacted object and it returns a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Url(url) => url.as_str(),
            Self::Memory(original) => match original {
                Some(original) => original,
                None => "sqlite::memory:",
            },
        }
    }

    /// Returns an in-memory `SqliteUrl` with no original string.
    #[must_use]
    pub const fn memory() -> Self {
        Self::Memory(None)
    }

    /// Returns `true` if this URL points to an in-memory database.
    #[must_use]
    pub const fn is_memory(&self) -> bool {
        matches!(self, Self::Memory { .. })
    }
}

impl Default for SqliteUrl {
    fn default() -> Self {
        Self::Memory(None)
    }
}

impl PartialEq for SqliteUrl {
    fn eq(&self, other: &Self) -> bool {
        use SqliteUrl::{Memory, Url};
        match (self, other) {
            (Url(this), Url(other)) => constant_time_eq(this.as_bytes(), other.as_bytes()),
            (Memory(Some(this)), Memory(Some(other))) => {
                constant_time_eq(this.as_bytes(), other.as_bytes())
            }
            (Memory(None), Memory(None)) => true,
            _ => false,
        }
    }
}

impl Eq for SqliteUrl {}

impl fmt::Display for SqliteUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(..) | Self::Memory(Some(..)) => f.write_str("<redacted>"),
            Self::Memory(None) => f.write_str(":memory:"),
        }
    }
}

struct SqliteUrlVisitor;

/// Returns `true` if the given SQLite in-memory URL contains query parameters
/// or authentication credentials that warrant retaining the original string.
fn memory_url_has_sensitive_parts(url: &str) -> bool {
    // Query parameters start with '?', credentials appear as user:pass@ in the URI.
    url.contains('?') || url.contains('@')
}

impl<'de> serde::de::Visitor<'de> for SqliteUrlVisitor {
    type Value = SqliteUrl;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an SQLite connection URL or \":memory:\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        /// Known in-memory SQLite connection string forms.
        const MEMORY_URLS: &[&str] = &[
            ":memory:",
            "sqlite::memory:",
            "sqlite://:memory:",
            "sqlite:///:memory:",
        ];

        let trimmed = v.trim();

        if MEMORY_URLS.iter().any(|m| trimmed.eq_ignore_ascii_case(m)) {
            return Ok(SqliteUrl::Memory(None));
        }

        // If the string looks like a memory URL with extra parts, preserve the
        // original so Display can redact it rather than silently dropping it.
        let is_memory_with_extras = MEMORY_URLS
            .iter()
            .any(|m| trimmed.to_ascii_lowercase().starts_with(*m))
            && memory_url_has_sensitive_parts(trimmed);

        if is_memory_with_extras {
            return Ok(SqliteUrl::Memory(Some(trimmed.to_string())));
        }

        if trimmed.is_empty() {
            return Err(E::custom("SQLite URL must not be empty"));
        }

        if !trimmed.starts_with("sqlite:") && !trimmed.starts_with('/') {
            return Err(E::custom(
                "invalid SQLite URL: must start with `sqlite:` or be an absolute path",
            ));
        }

        Ok(SqliteUrl::Url(Sensitive::new(trimmed.to_string())))
    }
}

impl<'de> Deserialize<'de> for SqliteUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SqliteUrlVisitor)
    }
}
