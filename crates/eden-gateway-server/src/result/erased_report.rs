use axum::response::{IntoResponse, Response};
use eden_sqlite::error::{ReportExt, SqlErrorType};
use error_stack::Report;
use std::fmt;
use std::sync::Arc;

use crate::result::ApiError;

/// A type-erased wrapper around [`Report`] that discards the context type parameter.
///
/// `error_stack::Report<C>` is generic over its context type `C`, which makes it
/// difficult to store reports of varying context types in a single field or pass them
/// through layers that shouldn't need to know the original error type. `OpaqueReport`
/// solves this by transmuting the context to `()`, leaving the internal frame chain
/// fully intact and inspectable via [`downcast_ref`](OpaqueReport::downcast_ref).
///
/// # Limitations
///
/// Calling [`Report::current_context`] on the inner report would be unsound since the
/// context type has been erased. This type intentionally does not expose that method.
pub struct ErasedReport {
    report: Report<()>,
}

impl ErasedReport {
    #[must_use]
    pub fn new<C>(report: Report<C>) -> Self
    where
        C: std::error::Error + Send + Sync + 'static,
    {
        // SAFETY: Report<C> and Report<()> are compatible to each other layout wise. The only field that
        //         differs and depends on C type which is `_context: PhantomData<fn() -> *const C>`, which
        //         it does not bring any significance during runtime. The `frames` field, which it actually
        //         holds the error chain, is fully opaque and identical across all [`Report`]'s with C types.
        //
        //         The resulting `Report<()>` must never call `current_context()`, as the erased context type
        //         would produce a dangling or misaligned reference. All other operations, including
        //         `downcast_ref` remain safe.
        let report: Report<()> = unsafe { std::mem::transmute(report) };
        Self { report }
    }

    /// Gets the [`SqlErrorType`] from the inner wrapped report.
    #[must_use]
    pub fn sql_error_type(&self) -> Option<SqlErrorType> {
        self.report.sql_error_type()
    }

    /// Wrapper of [`Report::downcast_ref`].
    #[must_use]
    pub fn downcast_ref<C>(&self) -> Option<&C>
    where
        C: Send + Sync + 'static,
    {
        self.report.downcast_ref()
    }
}

impl fmt::Debug for ErasedReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.report, f)
    }
}

impl fmt::Display for ErasedReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.report, f)
    }
}

impl<C> From<Report<C>> for ErasedReport
where
    C: std::error::Error + Send + Sync + 'static,
{
    fn from(report: Report<C>) -> Self {
        ErasedReport::new(report)
    }
}

impl IntoResponse for ErasedReport {
    fn into_response(self) -> Response {
        let mut res = ApiError::INTERNAL.into_response();
        res.extensions_mut().insert(Arc::new(self));
        res
    }
}

/// Extension trait for converting a `Result<T, Report<C>>` into `Result<T, ErasedReport>`,
/// erasing the context type parameter from the error.
///
/// This is useful when results of varying `Report<C>` types need to be stored,
/// returned, or passed through a layer that should not depend on a specific context type.
///
pub trait EraseReportExt<T> {
    fn erase_report(self) -> Result<T, ErasedReport>;
}

impl<T, C> EraseReportExt<T> for Result<T, Report<C>>
where
    C: std::error::Error + Send + Sync + 'static,
{
    fn erase_report(self) -> Result<T, ErasedReport> {
        self.map_err(ErasedReport::new)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_some;
    use error_stack::ResultExt;
    use std::hint::black_box;
    use thiserror::Error;

    use crate::result::{EraseReportExt, ErasedReport};

    #[derive(Debug, Error)]
    #[error("Could not parse configuration file")]
    struct ParseConfigError;

    #[allow(dead_code)]
    struct Suggestion(&'static str);

    fn produce_report() -> ErasedReport {
        std::fs::read_to_string("/")
            .change_context(ParseConfigError)
            .attach_opaque(Suggestion("use a file you can read next time!"))
            .attach_with(|| "hopefully it should not throw SIGFAULT to us")
            .erase_report()
            .unwrap_err()
    }

    #[test]
    fn can_use_downcast_ref() {
        let report = produce_report();
        let suggestion = report.downcast_ref::<Suggestion>();
        assert_some!(suggestion);

        let report = produce_report();
        let error = report.downcast_ref::<ParseConfigError>();
        assert_some!(error);
    }

    #[test]
    fn should_not_emit_segfault_in_debug() {
        black_box(format!("{:?}", produce_report()));
        black_box(format!("{:#?}", produce_report()));
    }

    #[test]
    fn should_not_emit_segfault_in_display() {
        black_box(format!("{}", produce_report()));
    }
}
