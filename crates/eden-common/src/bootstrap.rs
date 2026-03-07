use error_stack::Report;
use thiserror::Error;

/// Error returned when the rustls crypto provider cannot be installed.
#[derive(Debug, Error)]
#[error("Failed to initialize rustls crypto provider")]
pub struct InitRustlsError;

/// Installs the default [`ring`] crypto provider for [`rustls`].
///
/// **This function must be called preferably before every Eden-provided binary starts.**
///
/// [`ring`]: rustls::crypto::ring
pub fn init_rustls() -> Result<(), Report<InitRustlsError>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| Report::new(InitRustlsError))
}
