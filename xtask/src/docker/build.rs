use erased_report::ErasedReport;
use error_stack::ResultExt;
use std::path::{Path, PathBuf};
use xshell::{Shell, cmd};

use crate::{docker, flags};

const DEFAULT_PROFILE: &str = "release";
const DEFAULT_REPO: &str = "memothelemo/eden";
const DEFAULT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl flags::Build {
    pub(crate) fn run(self, sh: &Shell) -> Result<(), ErasedReport> {
        let docker_exec = docker::locate()?;
        let workspace_dir = crate::workspace_dir();
        docker::check_buildx_installation(&docker_exec, sh)?;

        let profile = self.profile.as_deref().unwrap_or(DEFAULT_PROFILE);
        let repository = self.repository.as_deref().unwrap_or(DEFAULT_REPO);
        let version = self.version.as_deref().unwrap_or(DEFAULT_VERSION);
        let image = format!("{repository}:{version}");

        log::debug!("args.profile = {profile:?}");
        log::debug!("args.repository = {repository:?}");
        log::debug!("args.version = {version:?}");
        log::debug!("docker.path = {}", docker_exec.display());
        log::debug!("workspace.path = {}", workspace_dir.display());

        let _handle = setup_stub_crates(sh, &workspace_dir)?;
        build_image(sh, &workspace_dir, &image, profile)?;

        Ok(())
    }
}

/// Invokes `docker buildx build` to produce the Eden image.
///
/// The Rust toolchain version is derived automatically from `rust-toolchain.toml`
/// rather than being pinned here.
fn build_image(
    sh: &Shell,
    workspace_dir: &Path,
    image: &str,
    profile: &str,
) -> Result<(), ErasedReport> {
    log::info!("Building Eden Docker image {image:?}...");

    cmd!(sh, "docker buildx build {workspace_dir}")
        .args(&["-t", image])
        .args(&["--build-arg", &format!("RUST_BUILD_PROFILE={profile}")])
        .run()
        .attach_with(|| format!("failed to build Docker image {image:?}"))?;

    log::info!("Docker image {image:?} built successfully");
    Ok(())
}

struct StubCratesHandle<'a> {
    path: PathBuf,
    sh: &'a Shell,
}

/// This function attempts to mirror the structure of the entire `crates` directory
/// by making all of the crates in the directory to have no actual content but only the
/// necessary to be compiled successfully, so Docker can cache dependencies without
/// invalidating it whenever source changes (except for any changes made to the Cargo manifest files).
fn setup_stub_crates<'s>(
    sh: &'s Shell,
    workspace_dir: &Path,
) -> Result<StubCratesHandle<'s>, ErasedReport> {
    let stub_crates_dir = workspace_dir.join("stub-crates");
    let crates_dir = workspace_dir.join("crates");
    log::info!("Setting up stub crates directory for Docker layer caching...");

    if stub_crates_dir.exists() {
        sh.remove_path(&stub_crates_dir)
            .attach("failed to remove stale stub-crates directory")?;
    }

    sh.create_dir(&stub_crates_dir)
        .attach("failed to create stub-crates directory")?;

    let entries = sh
        .read_dir(&crates_dir)
        .attach("failed to read crates directory")?;

    sh.change_dir(&stub_crates_dir);

    for entry in entries {
        let crate_name = entry
            .file_name()
            .expect("should provide file name in every directory entry");

        let stub_crate_dir = stub_crates_dir.join(crate_name);
        sh.create_dir(&stub_crate_dir)
            .attach_with(|| format!("failed to create stub crate directory for {crate_name:?}"))?;

        // Copy the real Cargo.toml so dependency metadata is preserved for
        // Docker's cache layer, but stub out the source so there's nothing
        // meaningful to compile beyond an empty library root.
        sh.copy_file(entry.join("Cargo.toml"), stub_crate_dir.join("Cargo.toml"))
            .attach_with(|| format!("failed to copy Cargo.toml for {crate_name:?}"))?;

        sh.write_file(stub_crate_dir.join("src").join("lib.rs"), "")
            .attach_with(|| format!("failed to write stub lib.rs for {crate_name:?}"))?;
    }

    Ok(StubCratesHandle {
        path: stub_crates_dir,
        sh,
    })
}

impl Drop for StubCratesHandle<'_> {
    fn drop(&mut self) {
        _ = self.sh.remove_path(&self.path);
        log::info!("Deleted stub crates directory");
    }
}
