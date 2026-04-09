use thiserror::Error;

mod instance;
mod macros;

pub use self::instance::InstanceMetrics;

#[derive(Debug, Error)]
#[error("Failed to encode prometheus metrics into string")]
pub struct EncodeError;
