use serde::Deserialize;

/// Configuration for the initial setup for Eden to operate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Setup {
    pub settings: InitialSettings,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            settings: InitialSettings::default(),
        }
    }
}

/// Initial settings to be set as default for new primary guilds
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InitialSettings {
    /// Whether to allow guests in the server, specifically players who
    /// are not joined as a member in the organization.
    pub allow_guests: bool,
}

impl Default for InitialSettings {
    fn default() -> Self {
        Self { allow_guests: true }
    }
}
