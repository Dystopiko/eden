use error_stack::{Report, ResultExt};
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

use std::fmt;
use std::ops;

use crate::error::*;
use crate::root::Config;

/// A handle to an Eden configuration file that supports both
/// reading and writing.
pub struct EditableConfig {
    /// Path to the config file, regardless if it exists or not.
    path: PathBuf,

    /// Typed representation of the config
    parsed: Config,

    /// The raw, unparsed TOML document
    document: DocumentMut,
}

impl EditableConfig {
    /// Creates an `EditableConfig` bound to `path` and pre-populated with the
    /// compiled-in default configuration.
    #[must_use]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let (parsed, document) = Config::defaults();

        Self {
            path,
            parsed: parsed.clone(),
            document: document.clone().into_mut(),
        }
    }
}

impl EditableConfig {
    /// Applies a modification to the in-memory configuration, then it tries to
    /// save these changes to the disk, and attempt to parse the configuration
    /// again if needed.
    ///
    /// This function allows you to safely modify the underlying TOML
    /// document using a closure.
    #[track_caller]
    pub fn edit(
        &mut self,
        callback: impl Fn(&Config, &mut DocumentMut),
    ) -> Result<(), Report<EditConfigError>> {
        let mut document = self.document.clone();
        callback(&self.parsed, &mut document);

        // Then, we can save changes to the specified path
        let document = document.to_string();
        eden_common::path::write_atomic(&self.path, &document).change_context(EditConfigError)?;

        let (parsed, document) =
            Config::load(&document, &self.path).change_context(EditConfigError)?;

        // Safely mutate the necessary fields
        self.parsed = parsed;
        self.document = document.into_mut();

        Ok(())
    }

    /// Opens an existing config file at `path`: parses it, validates it, and
    /// returns a ready-to-use `EditableConfig` on success.
    #[track_caller]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Report<ConfigLoadError>> {
        let mut this = Self::new(path);
        this.reload()?;
        Ok(this)
    }

    /// Reads the file at [`EditableConfig::path`] from disk, parses and
    /// validates it, and — on success — replaces the current in-memory state.
    ///
    /// The previous in-memory state is left untouched if any step fails, so a
    /// failed `reload` never leaves the `EditableConfig` in an inconsistent
    /// state.
    #[track_caller]
    pub fn reload(&mut self) -> Result<(), Report<ConfigLoadError>> {
        let source = eden_common::path::read(&self.path).change_context(ConfigLoadError)?;
        let (parsed, document) =
            Config::load(&source, &self.path).change_context(ConfigLoadError)?;

        // Only mutate state after every fallible step has succeeded.
        self.parsed = parsed;
        self.document = document.into_mut();

        Ok(())
    }

    /// Writes the current in-memory configuration to disk.
    #[track_caller]
    pub fn save(&self) -> Result<(), Report<SaveConfigError>> {
        eden_common::path::write_atomic(&self.path, self.document.to_string())
            .change_context(SaveConfigError)
    }
}

impl EditableConfig {
    /// Returns the raw TOML document of the config.
    ///
    /// Spans are not present in DocumentMut.
    #[must_use]
    pub fn document(&self) -> &DocumentMut {
        &self.document
    }

    /// Returns `true` if the config file already exists on disk.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Consumes the entire [`EditableConfig`] object and
    /// returns an owned [`Config`] object.
    #[must_use]
    pub fn into_inner(self) -> Config {
        self.parsed
    }

    /// Returns the filesystem path this `EditableConfig` is bound to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ops::Deref for EditableConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

impl fmt::Debug for EditableConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditableConfig")
            .field("path", &self.path)
            .field("parsed", &self.parsed)
            .finish_non_exhaustive()
    }
}
