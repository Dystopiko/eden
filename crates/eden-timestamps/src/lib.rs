use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{fmt::Display, time::Duration};

pub mod error;

use self::error::{TimestampParseError, TimestampParseErrorType};

/// A database-compatible type represents date and time in UTC, wrapped with
/// [`chrono::DateTime<Utc>`] internally.
///
/// # Formatting
/// Eden API timestamps are formatted as prescribed from [RFC 3339] or
/// `YYYY-MM-DDTHH:MM:SS.SSS+00:00` or any time zone available.
///
/// [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a new [`Timestamp`] with the current system date
    /// and time in UTC.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Parses a timestamp from an RFC 3339 date and time string.
    pub fn parse(input: &str) -> Result<Self, TimestampParseError> {
        DateTime::parse_from_rfc3339(input)
            .map(|v| Self(v.to_utc()))
            .map_err(|error| TimestampParseError {
                kind: TimestampParseErrorType::Format,
                source: Some(Box::new(error)),
            })
    }

    /// Creates a new [`Timestamp`] from the number of seconds since
    /// the Unix epoch (January 1st, 1970 at 00:00:00 UTC)
    pub fn from_secs(secs: i64) -> Result<Self, TimestampParseError> {
        Utc.timestamp_opt(secs, 0)
            .single()
            .map(Self)
            .ok_or_else(|| TimestampParseError {
                kind: TimestampParseErrorType::Range,
                source: None,
            })
    }

    /// Creates a new [`Timestamp`] from the number of milliseconds since
    /// the Unix epoch (January 1st, 1970 at 00:00:00 UTC)
    pub fn from_millis(millis: i64) -> Result<Self, TimestampParseError> {
        Utc.timestamp_millis_opt(millis)
            .single()
            .map(Self)
            .ok_or_else(|| TimestampParseError {
                kind: TimestampParseErrorType::Range,
                source: None,
            })
    }

    /// Creates a new [`Timestamp`] from the number of microseconds since
    /// the Unix epoch (January 1st, 1970 at 00:00:00 UTC)
    pub fn from_micros(millis: i64) -> Result<Self, TimestampParseError> {
        Utc.timestamp_micros(millis)
            .single()
            .map(Self)
            .ok_or_else(|| TimestampParseError {
                kind: TimestampParseErrorType::Range,
                source: None,
            })
    }
}

impl Timestamp {
    /// Returns the elapsed [duration] since the provided timestamp.
    ///
    /// It returns two types, the duration elaped since the provided
    /// timestamp, and whether it goes forward or backwards.
    ///
    /// It goes backward if the provided timestamp returns out to be
    /// later than the current system time, the second type will return
    /// `true`, otherwise it will return `false`.
    #[must_use]
    pub fn elapsed(&self) -> (Duration, bool) {
        let elapsed = self.0.signed_duration_since(Utc::now());
        let delta = elapsed
            .abs()
            .to_std()
            .expect("should provide std duration from non-negative time delta");

        (delta, elapsed.to_std().is_ok())
    }

    /// Returns the elapsed [duration] since the Unix epoch
    /// (January 1st, 1970 at 00:00:00 UTC) based on the provided timestamp.
    ///
    /// If the provided timestamp returns out to be earlier than the Unix epoch,
    /// it will return [`None`], but most of the cases, it will return [`Some`].
    ///
    /// [duration]: Duration
    #[must_use]
    pub fn elapsed_from_unix(&self) -> Option<Duration> {
        self.0
            .signed_duration_since(DateTime::UNIX_EPOCH)
            .to_std()
            .ok()
    }

    /// Returns the number of seconds since the Unix epoch
    /// (January 1st, 1970 at 00:00:00 UTC).
    #[must_use]
    pub const fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    /// Returns the number of milliseconds since the Unix epoch
    /// (January 1st, 1970 at 00:00:00 UTC).
    #[must_use]
    pub const fn timestamp_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
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
                Timestamp::parse(v).map_err(serde::de::Error::custom)
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
        let rfc_3339 = self.0.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Display::fmt(&rfc_3339, f)
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

#[cfg(test)]
mod tests {
    use crate::Timestamp;
    use claims::{assert_err, assert_ok};

    // Adopted from: https://github.com/twilight-rs/twilight/blob/5f6e4ae198fbd7a879e3eb5f58d133d0ee425b77/twilight-model/src/util/datetime/display.rs
    #[test]
    fn should_display_valid_rfc_3339() {
        const EXPECTED: &str = "2020-02-02T02:02:02.020Z";
        const TIME: i64 = 1_580_608_922_020_000;

        let timestamp = Timestamp::from_micros(TIME).expect("non zero");

        // Default formatter should be with microseconds.
        assert_eq!(EXPECTED, timestamp.to_string());
    }

    #[test]
    fn should_parse_valid_rfc_3339_timestamp() {
        static VALID_CASES: &[&str] = &[
            "2026-03-02T21:06:33Z",
            "2026-03-02T21:06:33+08:00",
            "2026-03-02T13:06:33.123456-08:00",
            "1990-12-31T23:59:60Z", // Leap second
            "2026-03-02t21:06:33z", // Lowercase
            "2026-03-02 21:06:33Z", // Should accept this but not recommended
        ];

        for input in VALID_CASES {
            let result = Timestamp::parse(input);
            assert_ok!(
                result,
                "{input:?} is a valid RFC 3339 timestamp but it failed to parse"
            );
        }
    }

    #[test]
    fn should_not_parse_invalid_rfc_3339_timestamp() {
        static INVALID_CASES: &[&str] = &[
            "2026-03-02T21:06:33",  // Missing Offset/Z
            "2026-02-30T21:06:33Z", // Non-existent date
            "2026-03-02T25:06:33Z", // Invalid hour
            "26-03-02T21:06:33Z",   // 2-digit year
        ];

        for input in INVALID_CASES {
            let result = Timestamp::parse(input);
            assert_err!(
                result,
                "{input:?} is not a valid RFC 3339 timestamp but it was successfully parsed"
            );
        }
    }

    #[test]
    fn should_not_parse_other_timestamp_formats() {
        static INVALID_CASES: &[&str] = &[
            "20260302T210633Z",                // ISO 8601 Basic
            "1772485593",                      // Unix Epoch
            "Mon, 02 Mar 2026 21:06:33 +0000", // RFC 2822
        ];

        for input in INVALID_CASES {
            let result = Timestamp::parse(input);
            assert_err!(
                result,
                "{input:?} is not a valid RFC 3339 timestamp but it was successfully parsed"
            );
        }
    }
}
