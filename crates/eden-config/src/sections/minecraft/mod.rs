use eden_validation::minecraft::validate_username;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use uuid::{Uuid, fmt::Hyphenated};

/// Configuration for the application's Minecraft server management.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Minecraft {
    /// Whether to allow guests in the server, specifically players who
    /// are not joined as a member in the organization.
    pub allow_guests: bool,

    /// These can be activated for contributors, members, and other designated
    /// players by setting up a list of supported permissions, allowing the
    /// client to utilize the LuckPerms API to apply the appropriate changes.
    pub perks: Perks,
}

impl Default for Minecraft {
    fn default() -> Self {
        Self {
            allow_guests: true,
            perks: Perks::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Perks {
    pub contributors: Vec<String>,
    pub member: Vec<String>,
    #[serde(flatten)]
    pub others: HashMap<UuidOrUsername, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum UuidOrUsername {
    Uuid(Uuid),
    Username(String),
}

struct UuidOrUsernameVisitor;

impl<'de> serde::de::Visitor<'de> for UuidOrUsernameVisitor {
    type Value = UuidOrUsername;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Minecraft UUID or username")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Ok(uuid) = Hyphenated::from_str(v) {
            Ok(UuidOrUsername::Uuid(uuid.into_uuid()))
        } else if validate_username(v).is_ok() {
            Ok(UuidOrUsername::Username(v.to_string()))
        } else {
            Err(serde::de::Error::custom(
                "invalid Minecraft UUID or username",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for UuidOrUsername {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(UuidOrUsernameVisitor)
    }
}

impl Serialize for UuidOrUsername {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Uuid(uuid) => uuid.as_hyphenated().serialize(serializer),
            Self::Username(username) => serializer.collect_str(username),
        }
    }
}
