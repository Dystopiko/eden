use error_stack::{Report, ResultExt};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when working with environment variables.
#[derive(Debug, Error)]
pub enum VarError {
    /// The environment variable is not set.
    #[error("{variable:?} environment variable is not set")]
    NotPresent { variable: String },

    /// The environment variable is not set.
    #[error("{variable:?} environment variable contains invalid UTF-8")]
    InvalidUTF8 { variable: String },

    /// The environment variable value could not be parsed into the target type.
    ///
    /// You can get the error information with the [`Report::downcast_ref`] function.
    ///
    /// [`Report::downcast_ref`]: error_stack::Report::downcast_ref
    #[error("failed to parse {variable:?} environment variable")]
    Parse { variable: String },

    /// Failed to load the `.env` file.
    #[error("failed to load environment file")]
    LoadEnvFile,
}

/// Loads environment variables from a `.env` file.
///
/// This function will search for a `.env` file in the current directory and its
/// parents, loading the variables from it, afterwards it may return the path of
/// where it is loaded from.
///
/// If a variable is already set in the environment, its value will not be changed.
#[track_caller]
pub fn load() -> VarResult<Option<PathBuf>> {
    match dotenvy::dotenv() {
        Ok(path) => Ok(Some(path)),
        Err(e) if e.not_found() => Ok(None),
        Err(e) => Err(Report::new(e).change_context(VarError::LoadEnvFile)),
    }
}

/// Retrieves an optional environment variable by key.
#[track_caller]
pub fn var(key: &str) -> VarResult<Option<String>> {
    match dotenvy::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(dotenvy::Error::EnvVar(std::env::VarError::NotPresent)) => Ok(None),
        Err(error) => Err(from_dotenvy_error(key, error)),
    }
}

/// Retrieves a required environment variable by key.
#[track_caller]
pub fn required_var(key: &str) -> VarResult<String> {
    dotenvy::var(key).map_err(|error| from_dotenvy_error(key, error))
}

/// Retrieves an optional environment variable and parses it into
/// a specified generic type.
#[track_caller]
pub fn var_parsed<T>(key: &str) -> VarResult<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let Some(value) = var(key)? else {
        return Ok(None);
    };

    let result = value.parse::<T>().map(Some);
    result.change_context_lazy(|| VarError::Parse {
        variable: key.to_string(),
    })
}

/// Retrieves a required environment variable and parses it into
/// a specified generic type.
#[track_caller]
pub fn required_var_parsed<T>(key: &str) -> VarResult<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match var_parsed(key)? {
        Some(value) => Ok(value),
        None => Err(Report::new(VarError::Parse {
            variable: key.to_string(),
        })),
    }
}

/// Convenient [`Result`] wrapper with [`Report<VarError`] inserted
/// automatically as an error type.
type VarResult<T> = Result<T, Report<VarError>>;

fn from_dotenvy_error(var: &str, error: dotenvy::Error) -> Report<VarError> {
    let variable = var.to_string();
    match error {
        dotenvy::Error::EnvVar(std::env::VarError::NotPresent) => {
            Report::new(VarError::NotPresent { variable })
        }
        dotenvy::Error::EnvVar(std::env::VarError::NotUnicode(..)) => {
            Report::new(VarError::InvalidUTF8 { variable })
        }
        inner => Report::new(inner).change_context(VarError::LoadEnvFile),
    }
}
