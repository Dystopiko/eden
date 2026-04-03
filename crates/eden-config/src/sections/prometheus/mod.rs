use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};

/// Configuration for the Prometheus integration
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Prometheus {
    pub ip: IpAddr,
    pub port: u16,
}

const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 10002;

impl Default for Prometheus {
    fn default() -> Self {
        Self {
            ip: DEFAULT_IP,
            port: DEFAULT_PORT,
        }
    }
}
