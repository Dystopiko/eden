pub mod migrations;
pub mod primary_guild;
pub mod snowflake;
pub mod timestamp;

use async_trait::async_trait;
use eden_sqlite::error::PoolError;
use eden_sqlite::{PooledConnection, Transaction};
use error_stack::Report;

pub use self::snowflake::Snowflake;
pub use self::timestamp::Timestamp;

/// A trait for managing database connection pools with primary/replica support.
///
/// This trait provides an abstraction over SQLite connection pools, supporting
/// both a required primary database and an optional replica database. It handles
/// connection acquisition with automatic fallback logic to maximize availability.
#[async_trait]
pub trait DatabasePools: Send + Sync {
    /// Returns a reference to the primary database connection pool.
    ///
    /// The primary pool is used for all write operations and as a fallback
    /// when the replica is unavailable or unhealthy.
    fn primary_db(&self) -> &eden_sqlite::Pool;

    /// Returns a reference to the replica database connection pool, if one is configured.
    ///
    /// The replica pool is used for read operations when available and healthy.
    /// Returns `None` if no replica has been configured, in which case reads
    /// will fall back to the primary database.
    fn replica_db(&self) -> Option<&eden_sqlite::Pool>;

    /// Acquires a write connection from the primary database as a transaction.
    ///
    /// This should be used for any operations that modify the database. It always
    /// targets the primary pool — replicas are never used for writes.
    #[tracing::instrument(skip_all, name = "db.write")]
    async fn db_write(&self) -> Result<Transaction<'static>, Report<PoolError>> {
        tracing::debug!("obtaining primary database connection...");
        self.primary_db().begin().await
    }

    /// Acquires a read connection, preferring the replica database if available.
    ///
    /// Connection selection follows this priority order:
    /// 1. **Replica** — used if configured and healthy.
    /// 2. **Primary** — used as a fallback if no replica is configured, or if
    ///    the replica reports itself as [`PoolError::Unhealthy`].
    ///
    /// This method is suitable for the majority of read-only queries in a
    /// primary/replica setup, since it offloads read traffic to the replica
    /// whenever possible.
    #[tracing::instrument(skip_all, name = "db.read")]
    async fn db_read(&self) -> Result<PooledConnection, Report<PoolError>> {
        let replica_db = self.replica_db();
        let Some(replica) = replica_db.as_ref() else {
            tracing::debug!("obtaining primary database connection...");
            return self.primary_db().acquire().await;
        };

        tracing::debug!("obtaining replica database connection...");
        match replica.acquire().await {
            Ok(conn) => Ok(conn),
            Err(error) => match error.current_context() {
                PoolError::Unhealthy => {
                    tracing::warn!(
                        ?error,
                        "replica database is unhealthy, falling back to primary"
                    );
                    self.primary_db().acquire().await
                }
                _ => Err(error),
            },
        }
    }

    /// Acquires a read connection, preferring the primary database over the replica.
    ///
    /// Connection selection follows this priority order:
    /// 1. **Primary** — always attempted first.
    /// 2. **Replica** — used as a fallback only if the primary reports itself as
    ///    [`PoolError::Unhealthy`] and a replica is configured.
    ///
    /// This is useful for read operations that require the most up-to-date data,
    /// such as reads that immediately follow a write, where replica lag would be
    /// unacceptable. Prefer [`db_read`] for general-purpose reads to reduce load
    /// on the primary.
    ///
    /// [`db_read`]: DatabasePools::db_read
    #[tracing::instrument(skip_all, name = "db.read_prefer_primary")]
    async fn db_read_prefer_primary(&self) -> Result<PooledConnection, Report<PoolError>> {
        tracing::debug!("obtaining primary database connection...");
        match self.primary_db().acquire().await {
            Ok(conn) => Ok(conn),
            Err(error) => {
                if let PoolError::Unhealthy = error.current_context()
                    && let Some(replica) = self.replica_db().as_ref()
                {
                    tracing::warn!(
                        ?error,
                        "primary database is unhealthy, falling back to replica"
                    );
                    return replica.acquire().await;
                };
                Err(error)
            }
        }
    }
}

#[async_trait]
impl DatabasePools for eden_sqlite::Pool {
    fn primary_db(&self) -> &eden_sqlite::Pool {
        self
    }

    fn replica_db(&self) -> Option<&eden_sqlite::Pool> {
        None
    }
}
