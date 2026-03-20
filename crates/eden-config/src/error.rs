use thiserror::Error;

/// Error returned when loading the Eden configuration fails.
#[derive(Debug, Error)]
#[error("Failed to load Eden configuration")]
pub struct ConfigLoadError;

/// Error returned when saving configuration to disk fails.
#[derive(Debug, Error)]
#[error("Failed to edit Eden configuration")]
pub struct EditConfigError;

/// Error returned when saving configuration to disk fails.
#[derive(Debug, Error)]
#[error("Failed to save Eden configuration")]
pub struct SaveConfigError;
