use serde::{Deserialize, Serialize};
use std::{convert::Infallible, fmt, str::FromStr};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
    Other(String),
}

const SURVIVAL_TAG: &str = "survival";
const CREATIVE_TAG: &str = "creative";
const ADVENTURE_TAG: &str = "adventure";
const SPECTATOR_TAG: &str = "spectator";

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self {
            GameType::Survival => SURVIVAL_TAG,
            GameType::Creative => CREATIVE_TAG,
            GameType::Adventure => ADVENTURE_TAG,
            GameType::Spectator => SPECTATOR_TAG,
            GameType::Other(n) => n,
        };
        f.write_str(tag)
    }
}

impl<'de> Deserialize<'de> for GameType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(GameTypeVisitor)
    }
}

struct GameTypeVisitor;

impl<'de> serde::de::Visitor<'de> for GameTypeVisitor {
    type Value = GameType;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Minecraft game type")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(GameType::from_str(v).expect("GameType::from_str is infallible"))
    }
}

impl FromStr for GameType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            SURVIVAL_TAG => Ok(Self::Survival),
            CREATIVE_TAG => Ok(Self::Creative),
            ADVENTURE_TAG => Ok(Self::Adventure),
            SPECTATOR_TAG => Ok(Self::Spectator),
            _ => Ok(Self::Other(s.to_string())),
        }
    }
}

impl Serialize for GameType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::minecraft::GameType;

    #[test]
    fn test_serialization() {
        let possible_game_types = &[
            GameType::Survival,
            GameType::Creative,
            GameType::Adventure,
            GameType::Spectator,
        ];
        insta::assert_json_snapshot!(possible_game_types);
    }
}
