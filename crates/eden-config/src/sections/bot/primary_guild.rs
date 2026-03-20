use serde::Deserialize;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

/// Configuration for the primary guild for Eden.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PrimaryGuild {
    pub id: Id<GuildMarker>,
}
