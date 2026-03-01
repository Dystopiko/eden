use bon::Builder;
use error_stack::Report;
use thiserror::Error;

/// Configuration for constructing a [`Pool`](crate::pool::Pool).
///
/// Use [`PoolConfig::builder()`] to construct this type. The builder's
/// `build()` method performs validation before returning the config.
///
/// # Example
/// ```rust
/// # use eden_sqlite::PoolConfig;
/// # fn main() {
/// let config = PoolConfig::builder()
///     .url("sqlite://app.db".to_string())
///     .max_connections(10)
///     .build()
///     .unwrap();
/// # }
/// ```
#[derive(Debug, Builder)]
#[builder(finish_fn(vis = "", name = "build_internal"))]
pub struct PoolConfig {
    /// The SQLite connection URL (e.g. `sqlite://path/to/db.sqlite` or `sqlite::memory:`).
    pub url: String,

    /// Minimum number of connections the pool will maintain at all times.
    /// Defaults to `1`.
    #[builder(default = 1)]
    pub(crate) min_connections: u32,

    /// Maximum number of connections the pool is allowed to open.
    /// Must be greater than zero. Defaults to `1`.
    #[builder(default = 1)]
    pub(crate) max_connections: u32,

    /// Whether connections should be opened in read-only mode.
    /// Defaults to `false`.
    #[builder(default = false)]
    pub readonly: bool,
}

impl PoolConfig {
    #[must_use]
    pub const fn min_connections(&self) -> u32 {
        self.min_connections
    }

    #[must_use]
    pub const fn max_connections(&self) -> u32 {
        self.max_connections
    }
}

/// Errors that can occur when building a [`PoolConfig`].
#[derive(Debug, Error)]
pub enum PoolConfigBuildError {
    #[error("`max_connections` must not be equal to zero")]
    MaxConnectionsZero,
}

impl<S: pool_config_builder::IsComplete> PoolConfigBuilder<S> {
    /// Validates and returns the completed [`PoolConfig`].
    #[must_use]
    pub fn build(self) -> Result<PoolConfig, Report<PoolConfigBuildError>> {
        let config = self.build_internal();
        if config.max_connections == 0 {
            return Err(Report::new(PoolConfigBuildError::MaxConnectionsZero));
        }
        Ok(config)
    }
}
