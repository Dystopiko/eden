use serde::Deserialize;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, UserMarker};

/// Configuration for the primary guild for Eden.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PrimaryGuild {
    /// Eden's primary guild/server's ID.
    pub id: Id<GuildMarker>,

    /// Designated channel for alerts
    pub alert_channel_id: Id<ChannelMarker>,

    /// Configuration for Chaos (chaosneco) auto-trigger
    #[serde(default)]
    pub chaos: ChaosNeco,
}

/// Configuration for Chaos (chaosneco)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ChaosNeco {
    /// A list of user IDs that are associated to Chaos (chaosneco)
    pub user_ids: Vec<Id<UserMarker>>,
}

impl Default for ChaosNeco {
    fn default() -> Self {
        Self {
            user_ids: Vec::new(),
        }
    }
}
