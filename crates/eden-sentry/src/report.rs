use std::{
    fs::File,
    io::{BufRead, BufReader},
    panic::Location,
    path::Path,
};

use erased_report::ErasedReport;
use error_stack::{Frame, FrameKind, Report, iter::Frames};
use itertools::Itertools;
use sentry::{
    Hub, Level,
    protocol::{Event, Exception, TemplateInfo},
    types::Uuid,
};

#[allow(private_interfaces)]
#[track_caller]
pub fn capture_report(report: &dyn ReportLike) -> Uuid {
    Hub::with_active(|hub| hub.capture_event(event_from_report(report)))
}

#[allow(private_interfaces)]
#[must_use]
pub fn event_from_report(report: &dyn ReportLike) -> Event<'static> {
    let message_with_ansi = report.message();
    let message = strip_ansi_escapes::strip_str(message_with_ansi);

    let mut exceptions = report
        .frames()
        .peekable()
        .batching(|frames| {
            frames.peek()?;
            Some(exception_from_group(frames))
        })
        .collect::<Vec<_>>();

    exceptions.reverse();

    Event {
        level: Level::Error,
        message: Some(message),
        exception: exceptions.into(),
        template: report.location().map(create_template),
        ..Default::default()
    }
}

fn exception_from_group<'a>(frames: impl Iterator<Item = &'a Frame>) -> Exception {
    let mut ty = String::new();
    let mut value = String::new();

    for frame in frames {
        match frame.kind() {
            FrameKind::Context(context) => {
                ty = format!("{context:?}");
                if ty.starts_with('"') {
                    ty = "Custom".to_owned();
                }
                value = context.to_string();
                break;
            }
            FrameKind::Attachment(_) => {}
        }
    }

    Exception {
        ty,
        value: Some(value),
        ..Default::default()
    }
}

// Copied from: https://github.com/hashintel/hash/blob/7d41825862c0089c4ec5f73d533bf32e2bfc743a/libs/%40local/telemetry/src/traces/sentry.rs#L188-L243
fn read_source(location: Location) -> (Vec<String>, Option<String>, Vec<String>) {
    let Ok(file) = File::open(location.file()) else {
        return (Vec::new(), None, Vec::new());
    };

    // Extract relevant lines.
    let reader = BufReader::new(file);
    let line_no = location.line() as usize - 1;
    let start_line = line_no.saturating_sub(10);

    // Read the surrounding lines of `location`:
    // - 10 lines before (stored into `pre_context`)
    // - 3 lines after (stored into `post_context`)
    // - the line of `location` (stored into `context_line`)
    let mut pre_context = Vec::with_capacity(10);
    let mut context_line = None;
    let mut post_context = Vec::with_capacity(3);

    for (current_line, line) in reader.lines().enumerate().skip(start_line) {
        let Ok(line) = line else {
            // If the file can only partially be read, we cannot use the source.
            return (Vec::new(), None, Vec::new());
        };
        if current_line < line_no {
            pre_context.push(line);
        } else if current_line == line_no {
            context_line.replace(line);
        } else if current_line <= line_no + 3 {
            post_context.push(line);
        } else {
            break;
        }
    }

    (pre_context, context_line, post_context)
}

fn create_template(location: Location) -> TemplateInfo {
    let (pre_context, context_line, post_context) = read_source(location);

    let path = Path::new(location.file());
    path.file_name()
        .map(|path| path.to_string_lossy().to_string());

    TemplateInfo {
        filename: path
            .file_name()
            .map(|path| path.to_string_lossy().to_string()),
        abs_path: Some(location.file().to_owned()),
        lineno: Some(u64::from(location.line())),
        colno: Some(u64::from(location.column())),
        pre_context,
        context_line,
        post_context,
    }
}
// end of copy

mod private {
    use error_stack::iter::Frames;
    use std::panic::Location;

    pub trait ReportLike {
        fn location(&self) -> Option<Location<'static>>;

        #[doc(hidden)]
        fn frames(&self) -> Frames<'_>;

        #[doc(hidden)]
        fn message(&self) -> String;
    }
}
use self::private::ReportLike;

impl<C> ReportLike for Report<C> {
    fn location(&self) -> Option<Location<'static>> {
        self.downcast_ref::<Location>().copied()
    }

    fn frames(&self) -> Frames<'_> {
        self.frames()
    }

    fn message(&self) -> String {
        format!("{self:#?}")
    }
}

impl ReportLike for ErasedReport {
    fn location(&self) -> Option<Location<'static>> {
        self.downcast_ref::<Location>().copied()
    }

    fn frames(&self) -> Frames<'_> {
        self.frames()
    }

    fn message(&self) -> String {
        format!("{self}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use erased_report::ErasedReport;
    use error_stack::ResultExt;
    use thiserror::Error;

    use crate::report::event_from_report;

    #[derive(Debug, Error)]
    #[error("Failed to query table")]
    struct QueryError;

    // This is to simulate an actual scenario if there's an error in
    // any of our Eden services (Discord client and Gateway API server).
    #[tokio::test]
    async fn test_event_from_report() {
        let pool = eden_sqlite::Pool::memory(None).await;

        let mut conn = pool.acquire().await.unwrap();
        let report = sqlx::query("H")
            .execute(&mut *conn)
            .await
            .change_context(QueryError)
            .attach("while trying to query H in the database")
            .unwrap_err();

        let mut event = event_from_report(&ErasedReport::from_report(report));
        event.timestamp = SystemTime::UNIX_EPOCH;

        insta::assert_json_snapshot!(event);
    }
}
