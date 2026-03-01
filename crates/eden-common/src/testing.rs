use error_stack::Report;
use error_stack::fmt::{Charset, ColorMode};
use std::error::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

pub fn init() {
    disable_fancy_error_output();
    init_for_test();
}

/// Checks that an [`error_stack::Report`] contains a specific pattern in its
/// debug output.
///
/// # Panics
///
/// Panics if `pattern` is not found anywhere in the formatted debug output of `error`.
pub fn expect_error_containing<C: Error>(error: Report<C>, pattern: &str) {
    let message = format!("{error:?}");
    if !message.contains(pattern) {
        panic!("Cannot find {pattern:?} in this error here:\n{error:?}");
    }
}

/// Configures [`error_stack`] for use in tests by switching to ASCII output
/// and disabling ANSI color codes.
fn disable_fancy_error_output() {
    Report::set_charset(Charset::Ascii);
    Report::set_color_mode(ColorMode::None);
}

fn init_for_test() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}
