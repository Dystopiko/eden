use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};

/// Configuration for the gateway server.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Gateway {
    pub ip: IpAddr,
    pub port: u16,
    pub shared_secret_token: SharedSecretToken,
}

// Inspired from a popular nature park in the Philippines
const DEFAULT_PORT: u16 = 7590;

impl Default for Gateway {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_PORT,
            shared_secret_token: SharedSecretToken::new(""),
        }
    }
}

/// A wrapper for a Eden gateway authorization token allocated in
/// the heap with debug implementation that redacts the entire string.
///
/// The user is responsible for handling the token and avoiding
/// the token from being exposed in the stack memory.
#[derive(Clone, Default)]
pub struct SharedSecretToken {
    inner: Box<str>,
}

impl SharedSecretToken {
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

impl PartialEq for SharedSecretToken {
    /// Compares tokens in constant time to prevent timing side-channels.
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(self.inner.as_bytes(), other.inner.as_bytes())
    }
}

impl PartialEq<&str> for SharedSecretToken {
    /// Compares tokens in constant time to prevent timing side-channels.
    fn eq(&self, other: &&str) -> bool {
        constant_time_eq(self.inner.as_bytes(), other.as_bytes())
    }
}

impl Eq for SharedSecretToken {}

impl std::fmt::Debug for SharedSecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Token").finish_non_exhaustive()
    }
}

impl std::fmt::Display for SharedSecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for SharedSecretToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SharedSecretToken;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Eden shared secret token string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(SharedSecretToken::new(v))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
