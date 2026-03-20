use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Dimension {
    Minecraft(McDimension),
    // TODO: Have a separate type for other dimension to really enforce
    //       the strong typed philosophy of this API.
    Other { scope: String, specifier: String },
}

impl Dimension {
    pub const OVERWORLD: Self = Self::Minecraft(McDimension::Overworld);
    pub const THE_NETHER: Self = Self::Minecraft(McDimension::Nether);
    pub const THE_END: Self = Self::Minecraft(McDimension::End);
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minecraft(kind) => write!(f, "minecraft:{kind}"),
            Self::Other { scope, specifier } => {
                write!(f, "{scope}:{specifier}")
            }
        }
    }
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DimensionVisitor)
    }
}

struct DimensionVisitor;

impl<'de> serde::de::Visitor<'de> for DimensionVisitor {
    type Value = Dimension;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Minecraft dimension resource")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Dimension::from_str(v).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Dimension {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

// https://minecraft.wiki/w/Dimension
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum McDimension {
    Overworld,
    Nether,
    End,
}

impl fmt::Display for McDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::End => f.write_str("the_end"),
            Self::Nether => f.write_str("the_nether"),
            Self::Overworld => f.write_str("overworld"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum DimensionParseError {
    UnknownMcDimension,
    MissingColon,
    MissingSpecifier,
    MissingScope,
    FoundIllegalCharacter,
}

impl fmt::Display for DimensionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FoundIllegalCharacter => f.write_str("found illegal character"),
            Self::MissingColon => f.write_str("missing ':'"),
            Self::MissingSpecifier => f.write_str("missing dimension specifier"),
            Self::MissingScope => f.write_str("missing dimension scope"),
            Self::UnknownMcDimension => f.write_str("unknown minecraft dimension"),
        }
    }
}

impl std::error::Error for DimensionParseError {}

// Allow lowercase ASCII letters, digits, and underscores.
//
// This composes of full set of characters valid in a Minecraft
// resource location path segment.
const fn is_valid_resource_location_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
}

impl FromStr for Dimension {
    type Err = DimensionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (scope, rest) = match s.find(':') {
            None => return Err(DimensionParseError::MissingColon),
            Some(i) => s.split_at(i),
        };

        if scope.is_empty() {
            return Err(DimensionParseError::MissingScope);
        }

        let specifier = &rest[1..];
        if specifier.is_empty() {
            return Err(DimensionParseError::MissingSpecifier);
        }

        if !scope.chars().all(is_valid_resource_location_char)
            || !specifier.chars().all(is_valid_resource_location_char)
        {
            return Err(DimensionParseError::FoundIllegalCharacter);
        }

        if scope == "minecraft" {
            let mc_dimension_type = match specifier {
                "overworld" => McDimension::Overworld,
                "the_nether" => McDimension::Nether,
                "the_end" => McDimension::End,
                _ => return Err(DimensionParseError::UnknownMcDimension),
            };
            return Ok(Self::Minecraft(mc_dimension_type));
        }

        Ok(Self::Other {
            scope: scope.to_string(),
            specifier: specifier.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let possible_values = &[
            Dimension::OVERWORLD,
            Dimension::THE_END,
            Dimension::THE_NETHER,
            Dimension::Other {
                scope: "hello".into(),
                specifier: "world".into(),
            },
        ];
        insta::assert_json_snapshot!(possible_values);
    }

    #[test]
    fn test_display_fmt() {
        assert_eq!(Dimension::OVERWORLD.to_string(), "minecraft:overworld");
        assert_eq!(Dimension::THE_NETHER.to_string(), "minecraft:the_nether");
        assert_eq!(Dimension::THE_END.to_string(), "minecraft:the_end");
        assert_eq!(
            Dimension::Other {
                scope: "hello".into(),
                specifier: "world".into()
            }
            .to_string(),
            "hello:world"
        );
    }

    #[test]
    fn should_parse_custom_dimensions() {
        let dim: Dimension = "mymod:custom_dim".parse().unwrap();
        let expected = Dimension::Other {
            scope: "mymod".into(),
            specifier: "custom_dim".into(),
        };

        assert_eq!(dim, expected);
    }

    #[test]
    fn should_parse_minecraft_dimensions() {
        let dim: Dimension = "minecraft:overworld".parse().unwrap();
        assert_eq!(dim, Dimension::OVERWORLD);

        let dim: Dimension = "minecraft:the_nether".parse().unwrap();
        assert_eq!(dim, Dimension::THE_NETHER);

        let dim: Dimension = "minecraft:the_end".parse().unwrap();
        assert_eq!(dim, Dimension::THE_END);
    }

    #[test]
    fn should_reject_parsing() {
        assert_eq!(
            "nodimension".parse::<Dimension>().unwrap_err(),
            DimensionParseError::MissingColon
        );
        assert_eq!(
            "a:b:c".parse::<Dimension>().unwrap_err(),
            DimensionParseError::FoundIllegalCharacter
        );
        assert_eq!(
            "minecraft:unknown".parse::<Dimension>().unwrap_err(),
            DimensionParseError::UnknownMcDimension
        );
        assert_eq!(
            "UPPER:case".parse::<Dimension>().unwrap_err(),
            DimensionParseError::FoundIllegalCharacter
        );
    }
}
