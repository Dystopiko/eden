use serde::Deserialize;

/// Configuration for the application's Minecraft server management.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Minecraft {
    /// Whether to allow guests in the server, specifically players who
    /// are not joined as a member in the organization.
    pub allow_guests: bool,
}

impl Default for Minecraft {
    fn default() -> Self {
        Self { allow_guests: true }
    }
}
