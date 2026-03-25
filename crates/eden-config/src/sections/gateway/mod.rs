use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};

/// Configuration for the gateway server.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Gateway {
    pub ip: IpAddr,
    pub port: u16,
}

// Inspired from a popular nature park in the Philippines
const DEFAULT_PORT: u16 = 7590;

impl Default for Gateway {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_PORT,
        }
    }
}
