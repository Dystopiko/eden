use error_stack::{Report, ResultExt};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Error returned when reading a file fails.
#[derive(Debug, Error)]
#[error("failed to read `{}`", .0.display())]
pub struct ReadFileError(PathBuf);

impl ReadFileError {
    /// Returns the path of the file that could not be read.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

/// Error returned when writing a file fails.
#[derive(Debug, Error)]
#[error("failed to write `{}`", .0.display())]
pub struct WriteFileError(PathBuf);

impl WriteFileError {
    /// Returns the path of the file that could not be written.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

/// Reads a file that may or may not exist, returning its contents as a `String`.
///
/// Returns `Ok(None)` when the file does not exist instead of an error.
pub fn read_optional(path: &Path) -> Result<Option<String>, Report<ReadFileError>> {
    let Some(bytes) = read_bytes_optional(path)? else {
        return Ok(None);
    };
    String::from_utf8(bytes).map(Some).map_err(|_| {
        Report::new(ReadFileError::new(path)).attach("file contains invalid UTF-8 content")
    })
}

/// Reads a file and returns its contents as a `String`.
///
/// Equivalent to [`std::fs::read_to_string`] but with richer error messages.
pub fn read(path: &Path) -> Result<String, Report<ReadFileError>> {
    String::from_utf8(read_bytes(path)?).map_err(|_| {
        Report::new(ReadFileError::new(path)).attach("file contains invalid UTF-8 content")
    })
}

/// Reads a file that may or may not exist, returning its raw bytes.
///
/// Returns `Ok(None)` when the file does not exist instead of an error.
pub fn read_bytes_optional(path: &Path) -> Result<Option<Vec<u8>>, Report<ReadFileError>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Report::new(error).change_context(ReadFileError::new(path))),
    }
}

/// Reads a file and returns its raw bytes.
///
/// Equivalent to [`std::fs::read`] but with richer error messages.
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, Report<ReadFileError>> {
    std::fs::read(path).change_context_lazy(|| ReadFileError::new(path))
}

/// Writes `contents` to `path`.
///
/// Equivalent to [`std::fs::write`] but with richer error messages.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<(), Report<WriteFileError>> {
    let path = path.as_ref();
    std::fs::write(path, contents.as_ref()).change_context_lazy(|| WriteFileError::new(path))
}

// Adapted from:
// https://github.com/rust-lang/cargo/blob/b9f0d83fd6528158af09d37e64779a0414da1ee2/crates/cargo-util/src/paths.rs#L76-L116
//
/// Normalizes a path by resolving `.` and `..` components without hitting the
/// filesystem.
///
/// **Caution**: unlike [`std::fs::canonicalize`], this does **not** resolve
/// symlinks, which may lead to unexpected behavior in some scenarios. Prefer
/// this helper when `canonicalize` would be overly strict (e.g. for paths that
/// do not exist yet, or to avoid verbose device paths on Windows).
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(Component::RootDir);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if ret.ends_with(Component::ParentDir) {
                    ret.push(Component::ParentDir);
                } else {
                    let popped = ret.pop();
                    if !popped && !ret.has_root() {
                        ret.push(Component::ParentDir);
                    }
                }
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

// Adapted from:
// https://github.com/rust-lang/cargo/blob/e05e4d726f66d074e78e635f5e745e31c8f621f4/crates/cargo-util/src/paths.rs#L190-L236
//
/// Writes `contents` to `path` atomically using a temporary file in the same
/// directory, then renaming it into place.
///
/// Preserves the existing file's Unix permission bits (user/group/other
/// read/write/execute) when replacing the file. The temporary file is created
/// with a restricted mode (`0o600`) and the permissions are updated before the
/// final rename, bypassing the process umask.
///
/// On Windows, the file is written via a named temporary file that is persisted
/// to the destination path.
pub fn write_atomic<C: AsRef<[u8]>>(
    path: &Path,
    contents: C,
) -> Result<(), Report<WriteFileError>> {
    // Follow symlinks so we replace the link target, not the symlink itself.
    let resolved_path;
    let path = if path.is_symlink() {
        resolved_path = std::fs::read_link(path)
            .change_context_lazy(|| WriteFileError::new(path))
            .attach("could not follow symlink of a file")?;

        &resolved_path
    } else {
        path
    };

    // On Unix platforms, get the permissions of the original file. Copy only the user/group/other
    // read/write/execute permission bits. The tempfile lib defaults to an initial mode of 0o600,
    // and we'll set the proper permissions after creating the file.
    #[cfg(unix)]
    let perms = path.metadata().ok().map(|meta| {
        use std::os::unix::fs::PermissionsExt;
        const PERMISSION_MASK: u32 = libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO;
        let mode = meta.permissions().mode() & PERMISSION_MASK;
        std::fs::Permissions::from_mode(mode)
    });

    let mut tmp = NamedTempFile::with_prefix_in(path.file_name().unwrap(), path.parent().unwrap())
        .change_context_lazy(|| WriteFileError::new(path))?;

    tmp.write_all(contents.as_ref())
        .change_context_lazy(|| WriteFileError::new(path))?;

    // On unix platforms, set the permissions on the newly created file. We can use fchmod (called
    // by the std lib; subject to change) which ignores the umask so that the new file has the same
    // permissions as the old file.
    #[cfg(unix)]
    if let Some(perms) = perms {
        tmp.as_file()
            .set_permissions(perms)
            .change_context_lazy(|| WriteFileError::new(path))
            .attach("could not set permissions on temporary file")?;
    }

    tmp.persist(path)
        .change_context_lazy(|| WriteFileError::new(path))?;

    Ok(())
}
