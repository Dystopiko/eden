use serde::Deserialize;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker};

/// Configuration for the primary guild for Eden.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PrimaryGuild {
    pub id: Id<GuildMarker>,
    pub alert_channel_id: Option<Id<ChannelMarker>>,
    pub contributor_role_id: Id<RoleMarker>,
    pub member_role_id: Id<RoleMarker>,
}
