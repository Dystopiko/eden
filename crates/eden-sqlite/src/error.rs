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
const SQLITE_READONLY: &str = "8";
const SQLITE_READONLY_ROLLBACK: &str = "776";

/// A high-level classification of a SQLite error, used to drive error handling
/// logic without requiring callers to inspect raw SQLite error codes directly.
#[derive(Debug, Clone)]
pub enum SqlErrorType {
    Unknown,
    UnhealthyConnection,
    RowNotFound,
    UniqueViolation(String),
    Readonly,
}

impl SqlErrorType {
    #[must_use]
    pub(crate) fn from_sqlx_error(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::WorkerCrashed => {
                SqlErrorType::UnhealthyConnection
            }
            sqlx::Error::RowNotFound => SqlErrorType::RowNotFound,
            sqlx::Error::Database(inner) => {
                Self::into_sqlite_error_type(inner.downcast_ref::<SqliteError>())
            }
            _ => SqlErrorType::Unknown,
        }
    }

    fn into_sqlite_error_type(err: &SqliteError) -> SqlErrorType {
        match err.code().as_deref() {
            // https://sqlite.org/rescode.html#constraint_unique
            Some(SQLITE_CONSTRAINT_UNIQUE) => {
                SqlErrorType::UniqueViolation(err.message().to_string())
            }
            // https://sqlite.org/rescode.html#cantopen
            Some(SQLITE_CANTOPEN) => SqlErrorType::UnhealthyConnection,
            // https://sqlite.org/rescode.html#readonly
            Some(SQLITE_READONLY) => SqlErrorType::Readonly,
            // https://sqlite.org/rescode.html#readonly_recovery
            Some(SQLITE_READONLY_ROLLBACK) => SqlErrorType::Readonly,
            _ => SqlErrorType::Unknown,
        }
    }
}

/// Extension trait that classifies a [`Report`] into a [`SqlErrorType`].
pub trait ReportExt {
    fn sql_error_type(&self) -> Option<SqlErrorType>;
}

/// Extension trait that classifies an [`SqlErrorType`] from a [`Result`] type.
pub trait ResultExt {
    fn sql_error_type(&self) -> Option<SqlErrorType>;
}

impl<E> ReportExt for Report<E> {
    fn sql_error_type(&self) -> Option<SqlErrorType> {
        if let Some(error) = self.downcast_ref::<sqlx::Error>() {
            Some(SqlErrorType::from_sqlx_error(error))
        } else {
            None
        }
    }
}

impl<T, E> ResultExt for Result<T, Report<E>> {
    fn sql_error_type(&self) -> Option<SqlErrorType> {
        let Err(error) = self else {
            return None;
        };
        error.sql_error_type()
    }
}

/// Extension trait that converts a [`Result`] from a database query into an
/// "optional" result, treating a [`SqlErrorType::RowNotFound`] error as a
/// successful absence of data rather than a failure.
///
/// This is useful when querying for a single row by primary key or unique
/// constraint, where "not found" is an expected and non-exceptional outcome.
pub trait QueryResultExt: Sized {
    /// The success type of the underlying `Result`.
    type Okay;

    /// The error context type wrapped inside the [`error_stack::Report`].
    type Err;

    /// Converts a `Result<Self::Okay, Report<Self::Err>>` into a
    /// `Result<Option<Self::Okay>, Report<Self::Err>>`.
    ///
    /// - If the result is `Ok(value)`, returns `Ok(Some(value))`.
    /// - If the result is an `Err` whose attached [`SqlErrorType`] is
    ///   [`SqlErrorType::RowNotFound`], returns `Ok(None)`.
    /// - Any other error is returned as-is.
    fn optional(self) -> Result<Option<Self::Okay>, Report<Self::Err>>;
}

impl<T, E> QueryResultExt for Result<T, Report<E>> {
    type Okay = T;
    type Err = E;

    fn optional(self) -> Result<Option<T>, Report<E>> {
        match self {
            // Query succeeded — wrap the value in Some.
            Ok(okay) => Ok(Some(okay)),
            // Query failed with RowNotFound — treat as a successful empty result.
            Err(..) if matches!(self.sql_error_type(), Some(SqlErrorType::RowNotFound)) => Ok(None),
            // Any other error — propagate as-is.
            Err(error) => Err(error),
        }
    }
}
