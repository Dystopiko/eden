use erased_report::ErasedReport;
use error_stack::ResultExt;
use std::path::Path;
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

        // Update the cargo lockfile since we're going to build the
        // entire Eden binary with `--locked` flag.
        cmd!(sh, "cargo update")
            .run()
            .attach("could not update Cargo.lock file")?;

        build_image(sh, &workspace_dir, &image, profile)
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
