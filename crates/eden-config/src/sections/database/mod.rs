use constant_time_eq::constant_time_eq;
use eden_common::Sensitive;
use eden_toml::TomlDiagnostic;
use error_stack::Report;
use serde::Deserialize;

use std::fmt;
use std::num::NonZeroU32;

use crate::validate::{Validate, ValidationContext};

/// Configuration for the application's SQLite database connections.
///
/// Supports a mandatory primary connection and an optional read replica,
/// allowing read-heavy workloads to be offloaded from the primary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct Database {
    /// The primary database pool, used for all reads and writes.
    #[serde(default)]
    pub primary: DatabasePool,

    /// An optional read-only replica pool.
    ///
    /// When present, read queries should be directed here to reduce
    /// load on the primary. It must be configured with `readonly: true`.
    pub replica: Option<DatabasePool>,
}

impl Validate for Database {
    fn validate(&self, ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>> {
        if let Some(replica) = self.replica.as_ref() {
            let replica_table = ctx.document.get("database").and_then(|v| v.get("replica"));
            let readonly_pair = replica_table
                .and_then(|v| v.as_table_like())
                .and_then(|v| v.get_key_value("readonly"));

            if !replica.readonly && readonly_pair.is_some() {
                let pair_span = readonly_pair
                    .and_then(|(key, value)| key.span().zip(value.span()))
                    .map(|(a, b)| a.start..b.end)
                    .or_else(|| replica_table.and_then(|v| v.span()));

                let diagnostic = eden_toml::diagnostic(
                    "Replica databases must not be writable. Set readonly to `false`",
                    pair_span,
                    ctx.source,
                    ctx.path,
                );

                return Err(diagnostic);
            }
        }

        Ok(())
    }
}

/// Configuration for a single SQLite connection pool.
///
/// Controls the connection URL, pool sizing, and whether the pool
/// should enforce read-only access at the connection level.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct DatabasePool {
    /// The SQLite connection URL for this pool.
    pub url: SqliteUrl,

    /// The minimum number of connections to keep alive in the pool.
    ///
    /// Connections are established eagerly up to this count on startup.
    /// Defaults to `0`, meaning no connections are held open proactively.
    pub min_connections: u32,

    /// The maximum number of connections the pool may open simultaneously.
    ///
    /// Requests that arrive when the pool is at capacity will wait until
    /// a connection becomes available. Defaults to `1`.
    pub max_connections: NonZeroU32,

    /// Whether to open all connections in read-only mode.
    ///
    /// When `true`, any attempt to write through this pool will be rejected
    /// by SQLite. Should always be `true` for replica pools.
    /// Defaults to `false`.
    pub readonly: bool,
}

impl Default for DatabasePool {
    fn default() -> Self {
        Self {
            url: SqliteUrl::default(),
            min_connections: 0,
            max_connections: NonZeroU32::new(1).expect("one is not zero"),
            readonly: false,
        }
    }
}

/// An SQLite connection URL, either file-backed or in-memory.
#[derive(Debug, Clone)]
pub enum SqliteUrl {
    /// A real file-backed or URI SQLite connection string.
    Url(Sensitive<String>),

    /// An in-memory SQLite database.
    ///
    /// `original` retains the raw string if it was parsed from a connection
    /// string (e.g. `":memory:"` or `"sqlite::memory:"`), primarily so that
    /// round-trip serialization and error messages can reference what the user
    /// actually wrote.
    Memory {
        /// The raw connection string, if it contained query parameters or
        /// credentials. `None` for plain in-memory URLs with no extras.
        original: Option<String>,
    },
}

impl SqliteUrl {
    /// Leaks the redacted object and it returns a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Url(url) => url.as_str(),
            Self::Memory { original } => match original {
                Some(original) => original,
                None => "sqlite::memory:",
            },
        }
    }

    /// Returns an in-memory `SqliteUrl` with no original string.
    #[must_use]
    pub const fn memory() -> Self {
        Self::Memory { original: None }
    }

    /// Returns `true` if this URL points to an in-memory database.
    #[must_use]
    pub const fn is_memory(&self) -> bool {
        matches!(self, Self::Memory { .. })
    }
}

impl Default for SqliteUrl {
    fn default() -> Self {
        Self::Memory { original: None }
    }
}

impl PartialEq for SqliteUrl {
    fn eq(&self, other: &Self) -> bool {
        use SqliteUrl::{Memory, Url};
        match (self, other) {
            (Url(this), Url(other)) => constant_time_eq(this.as_bytes(), other.as_bytes()),
            (
                Memory {
                    original: Some(this),
                },
                Memory {
                    original: Some(other),
                },
            ) => constant_time_eq(this.as_bytes(), other.as_bytes()),
            (Memory { original: None }, Memory { original: None }) => true,
            _ => false,
        }
    }
}

impl Eq for SqliteUrl {}

impl fmt::Display for SqliteUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(..) | Self::Memory { original: Some(..) } => f.write_str("<redacted>"),
            Self::Memory { original: None } => f.write_str(":memory:"),
        }
    }
}

struct Visitor;

/// Returns `true` if the given SQLite in-memory URL contains query parameters
/// or authentication credentials that warrant retaining the original string.
fn memory_url_has_sensitive_parts(url: &str) -> bool {
    // Query parameters start with '?', credentials appear as user:pass@ in the URI.
    url.contains('?') || url.contains('@')
}

impl<'de> serde::de::Visitor<'de> for Visitor {
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
            return Ok(SqliteUrl::Memory { original: None });
        }

        // If the string looks like a memory URL with extra parts, preserve the
        // original so Display can redact it rather than silently dropping it.
        let is_memory_with_extras = MEMORY_URLS
            .iter()
            .any(|m| trimmed.to_ascii_lowercase().starts_with(*m))
            && memory_url_has_sensitive_parts(trimmed);

        if is_memory_with_extras {
            return Ok(SqliteUrl::Memory {
                original: Some(trimmed.to_string()),
            });
        }

        if trimmed.is_empty() {
            return Err(E::custom("SQLite URL must not be empty"));
        }

        if !trimmed.starts_with("sqlite:") && !trimmed.starts_with('/') {
            return Err(E::custom(format!(
                "invalid SQLite URL: must start with `sqlite:` or be an absolute path"
            )));
        }

        Ok(SqliteUrl::Url(Sensitive::new(trimmed.to_string())))
    }
}

impl<'de> Deserialize<'de> for SqliteUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}
