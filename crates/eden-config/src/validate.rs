use eden_toml::TomlDiagnostic;
use error_stack::Report;
use std::path::Path;
use toml_edit::Document;

/// It provides context of the source configuration getting validated
/// after it is parsed, and essentially arguments for the
/// [`crate::toml::custom_diagnostic`] function.
pub struct ValidationContext<'a> {
    pub source: &'a str,
    pub path: &'a Path,
    pub document: &'a Document<String>,
}

/// This trait allows to validate a configuration section by validating
/// its own fields and recurisvely validating its nested fields whether
/// its corresponding constraints and requirements are met.
///
/// Each implementation of [`Validate`] is responsible for emitting
/// precise diagnostics (with correct source spans) when a constraint
/// is violated.
pub trait Validate {
    /// Checks all constraints for this section.
    ///
    /// Returns `Ok(())` when every field is valid, or a [`Report<TomlDiagnostic>`]
    /// pointing at the first offending field.
    fn validate(&self, ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>>;
}
