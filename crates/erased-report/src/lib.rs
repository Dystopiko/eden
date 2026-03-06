//! Type-erased wrapper around [`error_stack::Report`] that discards the context type parameter.
//!
//! # Overview
//!
//! [`error_stack::Report<C>`] is generic over its context type `C`, which makes it difficult to:
//!
//! - Store reports of varying context types in a single field (e.g. `Vec<Report<?>>`)
//! - Pass errors through layers that should not depend on a specific context type
//! - Return heterogeneous errors from a single function signature
//!
//! This crate provides [`ErasedReport`], a thin wrapper that transmutes the context type to `()`
//! while leaving the internal frame chain fully intact and inspectable. Downcasting, attachment,
//! and frame iteration all continue to work as expected.
//!
//! However, [`Report<[C]>`] or [`Report`] with multiple contexts are not
//! implemented at the moment.
//!
//! # Usage
//!
//! ## Erasing a `Report`
//!
//! ```rust
//! use error_stack::Report;
//! use erased_report::{ErasedReport, EraseReportExt};
//!
//! #[derive(Debug, thiserror::Error)]
//! #[error("something went wrong")]
//! struct MyError;
//!
//! let report: Report<MyError> = Report::new(MyError);
//! let erased: ErasedReport = ErasedReport::new(report);
//! ```
//!
//! ## Using the `EraseReportExt` convenience method
//!
//! ```rust
//! use erased_report::EraseReportExt;
//! use error_stack::ResultExt;
//!
//! #[derive(Debug, thiserror::Error)]
//! #[error("parse error")]
//! struct ParseError;
//!
//! let erased = std::fs::read_to_string("/nonexistent")
//!     .change_context(ParseError)
//!     .erase_report();
//! ```
//!
//! # Safety
//!
//! [`ErasedReport`] uses `unsafe` code internally to transmute [`Report<C>`] to [`Report<()>`].
//! This is sound because [`Report<C>`]'s layout does not materially differ across context
//! types at runtime — the only `C`-dependent field is a [`PhantomData`] marker, which carries
//! no runtime representation.
//!
//! The one operation that becomes unsound after erasure is [`Report::current_context`], which
//! [`ErasedReport`] intentionally does **not** expose. All other operations — including
//! downcasting, attachment, frame iteration, and `Display`/`Debug` formatting — remain safe.
//!
//! [`PhantomData`]: std::marker::PhantomData
use error_stack::{
    Attachment, OpaqueAttachment, Report,
    iter::{Frames, FramesMut},
};
use std::{error::Error as StdError, fmt};

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
    /// Creates a new [`ErasedReport`].
    ///
    /// Please refer to [`Report::new`] for how it works.
    #[expect(
        deprecated,
        reason = "error-stack still needs Context in some of their context-related functions"
    )]
    #[must_use]
    pub fn new<C>(context: C) -> Self
    where
        C: error_stack::Context,
    {
        Self::from_report(Report::new(context))
    }

    /// Converts [`Report<C>`] into [`ErasedReport`].
    #[expect(
        deprecated,
        reason = "error-stack still needs Context in some of their context-related functions"
    )]
    #[must_use]
    pub fn from_report<C>(report: Report<C>) -> Self
    where
        C: error_stack::Context,
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

    /// Wrapper of [`Report::downcast_ref`].
    #[must_use]
    pub fn downcast_ref<C>(&self) -> Option<&C>
    where
        C: Send + Sync + 'static,
    {
        self.report.downcast_ref()
    }

    /// Wrapper of [`Report::downcast_mut`].
    #[must_use]
    pub fn downcast_mut<C>(&mut self) -> Option<&mut C>
    where
        C: Send + Sync + 'static,
    {
        self.report.downcast_mut()
    }

    /// Wrapper of [`Report::into_error`].
    #[must_use]
    pub fn into_error(self) -> impl StdError + Send + Sync + 'static {
        self.report.into_error()
    }

    /// Returns this `Report` as an [`Error`].
    #[must_use]
    pub fn as_error(&self) -> &(impl StdError + Send + Sync + 'static) {
        self.report.as_error()
    }
}

impl ErasedReport {
    /// Wrapper of [`Report::attach`].
    #[track_caller]
    pub fn attach<A>(self, attachment: A) -> Self
    where
        A: Attachment,
    {
        let report = self.report.attach(attachment);
        Self { report }
    }

    /// Wrapper of [`Report::attach_opaque`].
    #[track_caller]
    pub fn attach_opaque<A>(self, attachment: A) -> Self
    where
        A: OpaqueAttachment,
    {
        let report = self.report.attach_opaque(attachment);
        Self { report }
    }

    /// Wrapper of [`Report::change_context`] but it retains the
    /// erased type of the Report.
    #[expect(
        deprecated,
        reason = "error-stack still needs Context in some of their context-related functions"
    )]
    #[must_use]
    pub fn push_context<T>(self, context: T) -> Self
    where
        T: error_stack::Context,
    {
        // SAFETY: See ErasedReport::new(...) for more info
        let report: Report<()> =
            unsafe { std::mem::transmute(self.report.change_context(context)) };
        Self { report }
    }

    /// Wrapper of [`Report::change_context`] but it converts
    /// back into a typed [`Report`].
    #[expect(
        deprecated,
        reason = "error-stack still needs Context in some of their context-related functions"
    )]
    #[must_use]
    pub fn change_context<T>(self, context: T) -> Report<T>
    where
        T: error_stack::Context,
    {
        // SAFETY: See ErasedReport::new(...) for more info
        self.report.change_context(context)
    }

    /// Wrapper of [`Report::frames`].
    #[must_use]
    pub fn frames(&self) -> Frames<'_> {
        self.report.frames()
    }

    /// Wrapper of [`Report::frames_mut`].
    #[must_use]
    pub fn frames_mut(&mut self) -> FramesMut<'_> {
        self.report.frames_mut()
    }

    /// Wrapper of [`Report::contains`].
    #[must_use]
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.report.contains::<T>()
    }
}

#[expect(
    deprecated,
    reason = "error-stack still needs Context in some of their context-related functions"
)]
impl<C: error_stack::Context> From<Report<C>> for ErasedReport {
    fn from(value: Report<C>) -> Self {
        ErasedReport::from_report(value)
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

impl From<ErasedReport> for Box<dyn StdError> {
    fn from(report: ErasedReport) -> Self {
        Box::new(report.into_error())
    }
}

impl From<ErasedReport> for Box<dyn StdError + Send> {
    fn from(report: ErasedReport) -> Self {
        Box::new(report.into_error())
    }
}

impl From<ErasedReport> for Box<dyn StdError + Sync> {
    fn from(report: ErasedReport) -> Self {
        Box::new(report.into_error())
    }
}

impl From<ErasedReport> for Box<dyn StdError + Send + Sync> {
    fn from(report: ErasedReport) -> Self {
        Box::new(report.into_error())
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
        self.map_err(ErasedReport::from_report)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_some;
    use error_stack::ResultExt;
    use std::hint::black_box;
    use thiserror::Error;

    use crate::{EraseReportExt, ErasedReport};

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
