use std::fmt;

/// An error type that carries a fully-rendered codespan diagnostic string.
#[derive(Debug)]
pub struct TomlDiagnostic(pub(crate) String);

impl fmt::Display for TomlDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for TomlDiagnostic {}
