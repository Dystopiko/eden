use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

/// Eden timestamps are formatted prescribed from [RFC 3339] or
/// `YYYY-MM-DDTHH:MM:SS.SSS+00:00`.
///
/// [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a [`Timestamp`] object based on the current time
    /// in the system in UTC.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.0
            .signed_duration_since(Utc::now())
            .abs()
            .to_std()
            .unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Eden timestamp")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timestamp::from_str(v).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.to_rfc3339(), f)
    }
}

impl FromStr for Timestamp {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DateTime::parse_from_rfc3339(s).map(|v| Self(v.to_utc()))
    }
}

impl<Tz: chrono::TimeZone> From<DateTime<Tz>> for Timestamp {
    fn from(value: DateTime<Tz>) -> Self {
        Self(value.to_utc())
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl From<Timestamp> for NaiveDateTime {
    fn from(value: Timestamp) -> Self {
        value.0.naive_utc()
    }
}

/// This module provides SQLx support for [`Timestamp`] type.
#[cfg(feature = "sqlx")]
mod sqlx;
