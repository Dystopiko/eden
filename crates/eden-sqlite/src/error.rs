use error_stack::Report;
use sqlx::error::DatabaseError;
use sqlx::sqlite::SqliteError;
use thiserror::Error;

/// Errors that can occur when constructing a [`Pool`].
#[derive(Debug, Error)]
pub enum PoolBuildError {
    #[error("Invalid SQLite connection URL")]
    InvalidConnectionURL,
}

/// Errors that can occur during pool operations such as acquiring a connection,
/// beginning a transaction, or checking pool health.
#[derive(Debug, Error)]
pub enum PoolError {
    /// The pool or an underlying connection is in a state where it cannot serve
    /// requests. Typically indicates a misconfigured path, a full pool, or a
    /// crashed worker. Callers may wish to surface this as a health failure.
    #[error("Failed to acquire pool connection")]
    General,

    /// The operation failed for a reason unrelated to pool health, such as a
    /// query error or an unexpected driver error.
    #[error("Pool is unhealthy")]
    Unhealthy,
}

/// SQLite extended result codes.
/// See: https://sqlite.org/rescode.html
const SQLITE_CONSTRAINT_UNIQUE: &str = "2067";
const SQLITE_CANTOPEN: &str = "14";

/// A high-level classification of a SQLite error, used to drive error handling
/// logic without requiring callers to inspect raw SQLite error codes directly.
#[derive(Debug)]
pub enum SqlErrorType {
    Unknown,
    UnhealthyConnection,
    UniqueViolation(String),
}

/// Extension trait that classifies a [`sqlx::Error`] into a [`SqlErrorType`].
pub trait SqlxErrorExt {
    fn sql_error_type(&self) -> SqlErrorType;
}

/// Extension trait that classifies an [`SqlErrorType`] from a [`Result`] type.
pub trait ResultExt {
    fn sql_error_type(&self) -> Option<&SqlErrorType>;
}

fn into_sqlite_error_type(err: &SqliteError) -> SqlErrorType {
    match err.code().as_deref() {
        // https://sqlite.org/rescode.html#constraint_unique
        Some(SQLITE_CONSTRAINT_UNIQUE) => SqlErrorType::UniqueViolation(err.message().to_string()),
        // https://sqlite.org/rescode.html#cantopen
        Some(SQLITE_CANTOPEN) => SqlErrorType::UnhealthyConnection,
        _ => SqlErrorType::Unknown,
    }
}

impl SqlxErrorExt for sqlx::Error {
    fn sql_error_type(&self) -> SqlErrorType {
        match self {
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::WorkerCrashed => {
                SqlErrorType::UnhealthyConnection
            }
            sqlx::Error::Database(inner) => {
                into_sqlite_error_type(inner.downcast_ref::<SqliteError>())
            }
            _ => SqlErrorType::Unknown,
        }
    }
}

impl<T> ResultExt for Result<T, Report<PoolError>> {
    fn sql_error_type(&self) -> Option<&SqlErrorType> {
        let Err(error) = self else {
            return None;
        };
        error.downcast_ref::<SqlErrorType>()
    }
}
