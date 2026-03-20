use serde::Deserialize;
use std::num::NonZeroU32;

use crate::sections::database::SqliteUrl;

/// Configuration for a single SQLite connection pool.
///
/// Controls the connection URL, pool sizing, and whether the pool
/// should enforce read-only access at the connection level.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct DatabasePool {
    pub url: SqliteUrl,
    pub min_connections: u32,
    pub max_connections: NonZeroU32,
    pub readonly: bool,
}

impl Default for DatabasePool {
    fn default() -> Self {
        Self {
            url: SqliteUrl::default(),
            min_connections: 0,
            max_connections: NonZeroU32::new(1).expect("one is not zero"),
            readonly: false,
        }
    }
}
