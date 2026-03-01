use error_stack::{Report, ResultExt};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use crate::config::PoolConfig;
use crate::error::{PoolBuildError, PoolError, SqlErrorType};

pub use sqlx::SqliteConnection as Connection;

/// A borrowed transaction on the SQLite pool.
pub type Transaction<'q> = sqlx::Transaction<'q, sqlx::Sqlite>;

/// A connection checked out from the pool. Returned to the pool on drop.
pub type PooledConnection = sqlx::pool::PoolConnection<sqlx::Sqlite>;

/// An asynchronous pool of database connections.
///
/// This object is a pointer of [`sqlx::SqlitePool`].
#[derive(Clone)]
pub struct Pool {
    inner: sqlx::SqlitePool,
}

impl Pool {
    /// Creates a new pool from the given [`PoolConfig`].
    ///
    /// The pool is created lazily so no connections are opened until
    /// the first operation is performed.
    pub fn new(config: PoolConfig) -> Result<Self, Report<PoolBuildError>> {
        let url = SqliteConnectOptions::from_str(&config.url)
            .change_context(PoolBuildError::InvalidConnectionURL)?
            .read_only(config.readonly);

        let inner = SqlitePoolOptions::new()
            .min_connections(config.min_connections)
            .max_connections(config.max_connections)
            .test_before_acquire(true)
            .connect_lazy_with(url);

        Ok(Self { inner })
    }

    /// Opens a pool backed by an in-memory SQLite database with optional
    /// configuration to setup maximum and minimum connections, and other
    /// options to customize the connection.
    ///
    /// If `config` is set to `None`, these values will set to its defaults:
    ///
    /// | Field             | Value   |
    /// |-------------------|---------|
    /// | `max_connections` | `100`   |
    /// | `min_connections` | `0`     |
    /// | `readonly`        | `false` |
    #[must_use]
    pub async fn memory(config: Option<&PoolConfig>) -> Self {
        let mut connect_opts = SqliteConnectOptions::from_str(":memory:")
            .expect("`:memory:` url should be a valid connection url");

        let mut pool_opts = SqlitePoolOptions::new()
            .max_connections(config.as_ref().map(|v| v.max_connections).unwrap_or(100));

        if let Some(config) = config.as_ref() {
            pool_opts = pool_opts.min_connections(config.min_connections);
            connect_opts = connect_opts.read_only(config.readonly);
        }

        let inner = pool_opts
            .connect_with(connect_opts)
            .await
            .expect("failed to connect memory pool");

        Self { inner }
    }
}

impl Pool {
    /// Acquires a connection from the pool, waiting if none are currently available.
    ///
    /// The connection is returned to the pool automatically when dropped.
    pub async fn acquire(&self) -> Result<PooledConnection, Report<PoolError>> {
        self.inner.acquire().await.map_err(classify_pool_err)
    }

    /// Begins a new database transaction, acquiring a connection from the pool.
    ///
    /// The transaction must be explicitly committed via [`Transaction::commit`];
    /// otherwise it will be rolled back on drop.
    pub async fn begin(&self) -> Result<Transaction<'static>, Report<PoolError>> {
        self.inner.begin().await.map_err(classify_pool_err)
    }

    /// Checks whether the pool can successfully acquire a connection and execute
    /// a trivial query (`SELECT 1`).
    ///
    /// Returns `true` if the probe succeeds, or `false` if the pool is unhealthy
    /// or the `timeout` elapses before the probe completes. If no timeout is
    /// provided, a default of 5 seconds is used.
    pub async fn check_health(&self, timeout: Option<Duration>) -> Result<bool, Report<PoolError>> {
        let timeout = timeout.unwrap_or(Duration::from_secs(5));
        tokio::time::timeout(timeout, self.probe())
            .await
            .unwrap_or(Ok(false))
    }

    /// Runs a lightweight probe query against a freshly acquired connection.
    ///
    /// Separated from [`Pool::check_health`] so that the timeout wrapper and
    /// the actual health logic each have a single responsibility.
    async fn probe(&self) -> Result<bool, Report<PoolError>> {
        let mut conn = match self.inner.acquire().await {
            Ok(conn) => conn,
            Err(error)
                if matches!(
                    SqlErrorType::from_sqlx_error(&error),
                    SqlErrorType::UnhealthyConnection
                ) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error).change_context(PoolError::General),
        };

        match sqlx::query("SELECT 1").execute(&mut *conn).await {
            Ok(..) => Ok(true),
            Err(error)
                if matches!(
                    SqlErrorType::from_sqlx_error(&error),
                    SqlErrorType::UnhealthyConnection
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error).change_context(PoolError::General),
        }
    }
}

impl Pool {
    /// Returns the current number of open connections in the pool,
    /// including both idle and in-use connections.
    #[must_use]
    pub fn connections(&self) -> u32 {
        self.inner.size()
    }

    /// Returns the number of connections currently sitting idle in the pool.
    #[must_use]
    pub fn idle_connections(&self) -> usize {
        self.inner.num_idle()
    }
}

impl fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl From<sqlx::SqlitePool> for Pool {
    fn from(inner: sqlx::SqlitePool) -> Self {
        Self { inner }
    }
}

fn classify_pool_err(error: sqlx::Error) -> Report<PoolError> {
    let context = match SqlErrorType::from_sqlx_error(&error) {
        SqlErrorType::UnhealthyConnection => PoolError::Unhealthy,
        _ => PoolError::General,
    };
    Report::new(error).change_context(context)
}
