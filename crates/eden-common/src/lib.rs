use error_stack::Report;
use thiserror::Error;

pub mod env;
pub mod path;
pub mod sensitive;
pub mod signals;
pub mod testing;

pub use self::sensitive::Sensitive;

/// Error returned when the rustls crypto provider cannot be installed.
#[derive(Debug, Error)]
#[error("Failed to initialize rustls crypto provider")]
pub struct InitRustlsError;

/// Installs the default [`aws_lc_rs`] crypto provider for [`rustls`].
///
/// **This function must be called preferably before every Eden-provided binary starts.**
///
/// [`aws_lc_rs`]: rustls::crypto::aws_lc_rs
pub fn init_rustls() -> Result<(), Report<InitRustlsError>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| Report::new(InitRustlsError))
}
