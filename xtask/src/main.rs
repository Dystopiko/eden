use erased_report::{ErasedReport, IntoErasedReportExt};
use std::{borrow::Cow, path::PathBuf};
use xshell::Shell;

mod docker;
mod flags;

fn main() -> Result<(), ErasedReport> {
    let dotenv = eden_utils::env::load().ok().flatten();
    let flags = match flags::Xtask::from_env() {
        Ok(flags) => flags,
        Err(error) if error.is_help() => {
            let error = error
                .to_string()
                .replace("{CARGO_PKG_VERSION}", env!("CARGO_PKG_VERSION"));

            println!("{error}");
            std::process::exit(0);
        }
        Err(error) => error.exit(),
    };

    let sh = &Shell::new().erase_report()?;
    sh.change_dir(workspace_dir());

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_module_path(false)
        .format_timestamp(None)
        .init();

    if let Some(dotenv) = dotenv {
        log::debug!("using dotenv file: {}", dotenv.display());
    }

    match flags.subcommand {
        flags::XtaskCmd::Docker(cmd) => cmd.run(sh),
    }
}

#[must_use]
fn stringify_output(bytes: &[u8]) -> Cow<'_, str> {
    // Windows uses UTF-16 for command outputs but...
    // https://en.wikipedia.org/wiki/Unicode_in_Microsoft_Windows#UTF-8
    String::from_utf8_lossy(bytes)
}

/// Returns the path to the root directory of Eden backend repository.
#[must_use]
fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
}
