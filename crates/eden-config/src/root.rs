use eden_common::env::var_parsed;
use eden_toml::TomlDiagnostic;
use error_stack::Report;
use serde::Deserialize;
use toml_edit::Document;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::sections::{Bot, Database};
use crate::validate::{Validate, ValidationContext};

/// The root configuration structure for Eden.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub bot: Bot,
    #[serde(default)]
    pub database: Database,
}

impl Config {
    #[must_use]
    pub fn suggest_path() -> PathBuf {
        Self::find().unwrap_or_else(|| PathBuf::from("eden.toml"))
    }

    #[must_use]
    pub fn find() -> Option<PathBuf> {
        const CANDIDATE_PATHS: &[&str] = &[
            "eden.toml",
            #[cfg(windows)]
            "%USERPROFILE%/.eden/config.toml",
            #[cfg(unix)]
            "/etc/eden/config.toml",
        ];

        // Prefer an explicit override from the environment first.
        let env_candidate = var_parsed::<PathBuf>("EDEN_SETTINGS").ok().flatten();
        if let Some(path) = env_candidate {
            return Some(path);
        }

        for candidate in CANDIDATE_PATHS {
            let path = PathBuf::from(candidate);
            if path.is_absolute() {
                if path.is_file() {
                    return Some(path);
                }
                continue;
            }

            // Walk up from the current directory looking for the file.
            let mut dir = std::env::current_dir().ok();
            while let Some(current) = dir.take() {
                let abs = current.join(&path);
                if abs.exists() {
                    return Some(abs);
                }
                dir = current.parent().map(Path::to_path_buf);
            }
        }

        None
    }
}

impl Config {
    /// Provides a static reference to the default typed [`Config`].
    #[must_use]
    pub fn no_clone_default() -> &'static Config {
        Self::defaults().0
    }

    /// Provides the default typed [`Config`] and the immutable [`Document`]
    /// from the bundled `eden.default.toml` found in the source code of the
    /// `eden-config` crate.
    pub(crate) fn defaults() -> (&'static Config, &'static Document<String>) {
        /// Lazily initialized default configuration parsed from the bundled `eden.default.toml`.
        static DEFAULTS: LazyLock<(Config, Document<String>)> = LazyLock::new(|| {
            (|| {
                let path = Path::new("<eden.default.toml>");
                let contents = include_str!("../eden.default.toml");

                let document = eden_toml::parse_document(contents, path)?;
                let config = eden_toml::deserialize(&document, path)?;
                Ok::<_, Report<TomlDiagnostic>>((config, document))
            })()
            .expect("bundled eden.default.toml must be a valid TOML document")
        });
        (&DEFAULTS.0, &DEFAULTS.1)
    }

    /// Parses and validates into typed [`Config`] from a source string.
    ///
    /// Returns the typed [`Config`] together with the immutable [`Document`] so
    /// that callers can store the document for later if needed.
    pub(crate) fn load(
        source: &str,
        path: &Path,
    ) -> Result<(Config, Document<String>), Report<TomlDiagnostic>> {
        let document = eden_toml::parse_document(source, path)?;

        let mut config: Self = eden_toml::deserialize(&document, path)?;
        config.validate(&ValidationContext {
            source,
            path,
            document: &document,
        })?;

        // Quick fix in `database.replica.readonly`
        if let Some(replica) = config.database.replica.as_mut() {
            let is_readonly_present = document
                .get("database")
                .and_then(|v| v.get("replica"))
                .and_then(|v| v.get("readonly"))
                .is_some();

            if !is_readonly_present && !replica.readonly {
                replica.readonly = true;
            }
        }

        Ok((config, document))
    }

    /// Runs every section's validator in sequence.
    ///
    /// It requires [validation context] since it requires the originating
    /// source of the config file to produce an elegant [TOML diagnostics] along
    /// with the relevant source line(s) via codespan.
    pub(crate) fn validate(
        &self,
        ctx: &ValidationContext<'_>,
    ) -> Result<(), Report<TomlDiagnostic>> {
        self.bot.validate(ctx)?;
        self.database.validate(ctx)?;
        Ok(())
    }
}
