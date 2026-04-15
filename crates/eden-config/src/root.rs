use eden_toml::TomlDiagnostic;
use eden_utils::env::var_parsed;
use error_stack::Report;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use toml_edit::Document;

use crate::{
    sections::{BackgroundJobs, Bot, Database, Gateway, Minecraft, Sentry, setup::Setup},
    validate::{Validate, ValidationContext},
};

/// The root configuration structure for Eden.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub background_jobs: BackgroundJobs,

    pub bot: Bot,

    #[serde(default)]
    pub database: Database,

    #[serde(default)]
    pub gateway: Gateway,

    #[serde(default)]
    pub minecraft: Minecraft,

    #[serde(default)]
    pub prometheus: bool,

    #[serde(default)]
    pub setup: Setup,

    pub sentry: Option<Sentry>,
}

impl Config {
    pub const FILE_NAME: &str = "eden.toml";

    #[must_use]
    pub fn find() -> Option<PathBuf> {
        const CANDIDATE_PATHS: &[&str] = &[
            Config::FILE_NAME,
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

    #[must_use]
    pub const fn template() -> &'static str {
        include_str!("../eden.template.toml")
    }
}

impl Config {
    pub(crate) fn maybe_toml_file(
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

    pub(crate) fn validate(
        &self,
        ctx: &ValidationContext<'_>,
    ) -> Result<(), Report<TomlDiagnostic>> {
        self.bot.validate(ctx)?;
        self.database.validate(ctx)?;
        self.gateway.validate(ctx)?;
        self.sentry.validate(ctx)?;
        Ok(())
    }
}
