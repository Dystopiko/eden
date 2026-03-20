use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term::Config,
};
use error_stack::Report;
use serde::de::DeserializeOwned;
use toml_edit::Document;

use std::fmt;
use std::ops::Range;
use std::path::Path;

mod error;
pub use self::error::TomlDiagnostic;

/// Parses a raw string with possibly valid TOML document into a
/// [TOML document] which it has the original source and spans to
/// easily identify errors.
///
/// [TOML document]: Document
pub fn parse_document(
    contents: &str,
    path: &Path,
) -> Result<Document<String>, Report<TomlDiagnostic>> {
    toml_edit::Document::parse(contents.to_owned())
        .map_err(|error| into_diagnostic(error.into(), contents, path))
}

/// Deserializes a value of type `T` from a [TOML document].
///
/// [TOML document]: Document
pub fn deserialize<T: DeserializeOwned>(
    document: &Document<String>,
    path: &Path,
) -> Result<T, Report<TomlDiagnostic>> {
    toml_edit::de::from_document(document.clone())
        .map_err(|error| into_diagnostic(error, document.raw(), path))
}

/// Converts a [`toml_edit::de::Error`] into a [`Report<TomlDiagnostic>`].
pub fn into_diagnostic(
    error: toml_edit::de::Error,
    source: &str,
    path: &Path,
) -> Report<TomlDiagnostic> {
    let span = error.span();
    let message = error.message().to_owned();
    diagnostic(message, span, source, path)
}

/// Builds a [`Report<TomlDiagnostic>`] from a plain message and an optional
/// byte-range span, rendering the relevant source line(s) via codespan.
pub fn diagnostic(
    message: impl fmt::Display,
    span: Option<Range<usize>>,
    source: &str,
    path: &Path,
) -> Report<TomlDiagnostic> {
    let file = SimpleFile::new(path.to_string_lossy(), source);

    let mut diagnostic = Diagnostic::error().with_message(message);
    let config = Config::default();

    if let Some(span) = span {
        let label = Label::primary((), span);
        diagnostic = diagnostic.with_label(label);
    }

    let inner = codespan_reporting::term::emit_into_string(&config, &file, &diagnostic)
        .expect("failed to emit diagnostic to string");

    Report::new(TomlDiagnostic(inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_emit() {
        let contents: &str = "hello";
        let diagnostic = parse_document(contents, Path::new("<test-emit>")).unwrap_err();
        insta::assert_snapshot!(diagnostic);
    }
}
