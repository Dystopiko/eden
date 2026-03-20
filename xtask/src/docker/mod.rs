use erased_report::{ErasedReport, IntoErasedReportExt};
use error_stack::{Report, ResultExt};
use std::path::{Path, PathBuf};
use thiserror::Error;
use which::which;
use xshell::{Shell, cmd};

use crate::{flags, stringify_output};

mod build;

impl flags::Docker {
    pub(crate) fn run(self, sh: &Shell) -> Result<(), ErasedReport> {
        match self.subcommand {
            flags::DockerCmd::Build(cmd) => cmd.run(sh),
        }
    }
}

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker is not installed")]
    NotInstalled,
}

#[derive(Debug, Error)]
#[error("docker-buildx is not installed")]
pub struct BuildxNotInstalled;

/// Attempts to locate the Docker binary based on the `PATH` environment variable.
pub fn locate() -> Result<PathBuf, Report<DockerError>> {
    which("docker")
        .change_context(DockerError::NotInstalled)
        .attach("Make sure Docker is installed in your system or development environment")
}

/// Attempts to check for `docker buildx` installation.
pub fn check_buildx_installation(docker_exec: &Path, sh: &Shell) -> Result<(), ErasedReport> {
    const BUILDX_INSTALL_SUGGESTION: &str = "Make sure to install docker-buildx or its equivalent package in your distribution or preferred package manager.";
    const BUILDX_NOT_COMMAND_ERR: &str = "docker: 'buildx' is not a docker command.";

    let output = cmd!(sh, "{docker_exec} buildx").output().erase_report()?;
    if output.status.success() {
        return Ok(());
    }

    let no_buildx = stringify_output(&output.stdout).contains(BUILDX_NOT_COMMAND_ERR)
        || stringify_output(&output.stderr).contains(BUILDX_NOT_COMMAND_ERR);

    if no_buildx {
        Err(ErasedReport::new(BuildxNotInstalled).attach(BUILDX_INSTALL_SUGGESTION))
    } else {
        Ok(())
    }
}
