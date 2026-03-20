//! Type-erased wrapper around [`error_stack::Report`] that discards the context type parameter.
//!
//! It provides this with [`ErasedReport`], a thin wrapper of [`error_stack::Report`]
//! that transmutes the context type to any context type, while leaving the internal
//! frame chain fully intact and inspectable.
//!
//! Downcasting, attachment, and frame iteration functions are mostly implemented
//! but reports stored with an array of [`Context`]'s are not implemented.
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
//! let erased: ErasedReport = ErasedReport::from_report(report);
//! ```
#![expect(
    deprecated,
    reason = "error-stack requires and uses Context trait for some of its functions and to make
              it compatible for virtually any Context implemented struct in erased-report."
)]

use error_stack::{
    Attachment, Context, OpaqueAttachment, Report,
    iter::{Frames, FramesMut},
};
use std::{error::Error as StdError, fmt};

/// A type-erased wrapper around [`Report`] that discards the context type parameter.
///
/// `error_stack::Report<C>` is generic over its context type `C`, which makes it
/// difficult to store reports of varying context types in a single field or pass them
/// through layers that shouldn't need to know the original error type.
#[must_use]
pub struct ErasedReport {
    report: Report<()>,
}

impl ErasedReport {
    /// A wrapper of [`Report::new`], except it returns [`ErasedReport`] where its
    /// context type is completely opaque.
    ///
    /// # Safety
    ///
    /// Please [`ErasedReport::from_report`] for more information on how it works internally.
    #[track_caller]
    pub fn new<C>(context: C) -> Self
    where
        C: Context,
    {
        Self::from_report(Report::new(context))
    }

    /// Wipes the required generic part of the [`error_stack::Report`] and returns
    /// the [`ErasedReport`] object where its context type is completely opaque.
    ///
    /// It simply converts from [`Report<C>`] into [`ErasedReport`].
    ///
    /// # Safety
    ///
    /// This function transmutes the [`Report<C>`] into [`Report<()>`].
    ///
    /// `Report<C>` and `Report<()>` are compatible to each other layout wise. The only field that
    /// differs and depends on `C` type which is `_context: PhantomData<fn() -> *const C>`.
    ///
    /// The `frames` field, which it actually holds the error chain, is fully opaque and identical
    /// across all [`Report`]'s with `C` types.
    #[allow(unsafe_code)]
    #[track_caller]
    pub fn from_report<C>(report: Report<C>) -> Self
    where
        C: Context,
    {
        // SAFETY:
        // The rest of it are found in the documentation of `ErasedReport::from_report`.
        //
        // The resulting `Report<()>` must never call `current_context()`, as the erased context
        // type would produce a dangling or misaligned reference. All other operations, including
        // `downcast_ref` remain safe.
        Self {
            report: unsafe { std::mem::transmute::<Report<C>, Report<()>>(report) },
        }
    }
}

impl ErasedReport {
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

    /// Wrapper of [`Report::frames`].
    pub fn frames(&self) -> Frames<'_> {
        self.report.frames()
    }

    /// Wrapper of [`Report::frames_mut`].
    pub fn frames_mut(&mut self) -> FramesMut<'_> {
        self.report.frames_mut()
    }

    /// Wrapper of [`Report::contains`].
    #[must_use]
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.report.contains::<T>()
    }
}

impl ErasedReport {
    /// Wrapper of [`Report::attach`].
    pub fn attach<A>(self, attachment: A) -> Self
    where
        A: Attachment,
    {
        Self {
            report: self.report.attach(attachment),
        }
    }

    /// Wrapper of [`Report::attach_opaque`].
    #[track_caller]
    pub fn attach_opaque<A>(self, attachment: A) -> Self
    where
        A: OpaqueAttachment,
    {
        Self {
            report: self.report.attach_opaque(attachment),
        }
    }

    /// Wrapper of [`Report::change_context`] but it still returns
    /// [`ErasedReport`].
    ///
    /// To change it back to [`Report<C>`], please use [`ErasedReport::change_context`].
    #[track_caller]
    pub fn push_context<T>(self, context: T) -> Self
    where
        T: error_stack::Context,
    {
        // SAFETY: See ErasedReport::new(...)
        Self::from_report(self.report.change_context(context))
    }

    /// Wrapper of [`Report::change_context`] but it converts
    /// back into a typed [`Report<C>`].
    ///
    /// To retain the [erased report] type, please use [`ErasedReport::push_context`].
    ///
    /// [erased report]" ErasedReport"
    #[track_caller]
    pub fn change_context<T>(self, context: T) -> Report<T>
    where
        T: error_stack::Context,
    {
        // SAFETY: See ErasedReport::new(...) for more info
        self.report.change_context(context)
    }
}

impl<C: Context> From<Report<C>> for ErasedReport {
    #[track_caller]
    fn from(report: Report<C>) -> Self {
        ErasedReport::from_report(report)
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

/// Extension trait for converting a `Result<T, C>` into `Result<T, ErasedReport>`,
/// erasing the context type parameter from the error.
///
/// This is useful when results of varying `C` types need to be stored, returned,
/// or passed through a layer that should not depend on a specific context type.
///
pub trait IntoErasedReportExt<T> {
    fn erase_report(self) -> Result<T, ErasedReport>;
}

impl<T, C> IntoErasedReportExt<T> for Result<T, C>
where
    C: Context,
{
    fn erase_report(self) -> Result<T, ErasedReport> {
        self.map_err(ErasedReport::new)
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
    C: Context,
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
