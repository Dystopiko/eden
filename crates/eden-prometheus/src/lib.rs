use thiserror::Error;

mod instance;
pub use self::instance::InstanceMetrics;

#[derive(Debug, Error)]
#[error("Failed to encode prometheus metrics into string")]
pub struct EncodeMetricsError;
