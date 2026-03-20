use error_stack::{Report, ResultExt};
use std::path::{Path, PathBuf};
use toml_edit::{Document, DocumentMut};

use crate::{
    Config,
    error::{ConfigLoadError, EditConfigError, SaveConfigError},
};

/// A handle to a configuration file that supports both reading and writing.
pub struct EditableConfig {
    /// Path to the config file, regardless if it exists or not.
    path: PathBuf,

    /// Typed representation of the config
    parsed: Config,

    /// The raw, unparsed TOML document
    document: DocumentMut,
}

impl EditableConfig {
    #[track_caller]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Report<ConfigLoadError>> {
        let path = path.as_ref();
        let (parsed, document) = read_from_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            parsed,
            document: document.into_mut(),
        })
    }

    #[track_caller]
    pub fn save_template<P: AsRef<Path>>(path: P) -> Result<(), Report<SaveConfigError>> {
        eden_utils::path::write_atomic(path.as_ref(), Config::template())
            .change_context(SaveConfigError)
    }
}

impl EditableConfig {
    #[track_caller]
    pub fn edit(
        &mut self,
        callback: impl Fn(&Config, &mut DocumentMut),
    ) -> Result<(), Report<EditConfigError>> {
        let mut document = self.document.clone();
        callback(&self.parsed, &mut document);

        // Then, we can save changes to the specified path
        let document = document.to_string();
        eden_utils::path::write_atomic(&self.path, &document).change_context(EditConfigError)?;

        let (parsed, document) = read_from_file(&self.path).change_context(EditConfigError)?;

        // Safely mutate the necessary fields
        self.parsed = parsed;
        self.document = document.into_mut();

        Ok(())
    }

    #[track_caller]
    pub fn reload(&mut self) -> Result<(), Report<ConfigLoadError>> {
        let (parsed, document) = read_from_file(&self.path).change_context(ConfigLoadError)?;

        // Only mutate state after every fallible step has succeeded.
        self.parsed = parsed;
        self.document = document.into_mut();

        Ok(())
    }

    #[track_caller]
    pub fn save(&self) -> Result<(), Report<SaveConfigError>> {
        eden_utils::path::write_atomic(&self.path, self.document.to_string())
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

impl std::ops::Deref for EditableConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

impl std::fmt::Debug for EditableConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditableConfig")
            .field("path", &self.path)
            .field("parsed", &self.parsed)
            .finish_non_exhaustive()
    }
}

fn read_from_file(path: &Path) -> Result<(Config, Document<String>), Report<ConfigLoadError>> {
    let source = eden_utils::path::read(path).change_context(ConfigLoadError)?;
    Config::maybe_toml_file(&source, path).change_context(ConfigLoadError)
}
