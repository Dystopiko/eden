use pin_project_lite::pin_project;
use std::{
    panic::AssertUnwindSafe,
    pin::Pin,
    task::{Context, Poll},
};

/// Extension trait that adds panic-catching capability to any
/// [`Future`].
///
/// This trait is automatically implemented for all futures via a
/// blanket impl, so you can call [`.catch_unwind()`](CatchUnwind::catch_unwind)
/// on any future to wrap it in a [`CatchUnwindFuture`].
pub trait CatchUnwind: Future
where
    Self: Sized,
{
    /// Wraps this future in a [`CatchUnwindFuture`], which catches panics and
    /// returns them as [`Err`] values rather than unwinding the stack.
    fn catch_unwind(self) -> CatchUnwindFuture<Self>;
}

impl<F: Future> CatchUnwind for F {
    fn catch_unwind(self) -> CatchUnwindFuture<Self> {
        CatchUnwindFuture::new(self)
    }
}

pin_project! {
    /// A future that catches panics from the wrapped future and returns them as [`Err`].
    ///
    /// This struct is created by the [`CatchUnwind::catch_unwind`] method. See its
    /// documentation for more details.
    ///
     /// # Unwind Safety
    ///
    /// This type uses [`AssertUnwindSafe`] internally, which bypasses Rust's unwind
    /// safety checks. **Callers are responsible** for ensuring that any shared state
    /// accessible by the wrapped future is not left in an inconsistent state after a
    /// panic. If the future touches `Mutex`-guarded state or other shared resources,
    /// those should be reviewed for logical unwind safety before using this wrapper.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct CatchUnwindFuture<F> {
        #[pin]
        future: F,
    }
}

impl<F: Future> CatchUnwindFuture<F> {
    /// Creates a new `CatchUnwindFuture` wrapping the given future.
    ///
    /// Prefer calling [`.catch_unwind()`](CatchUnwind::catch_unwind)
    /// on the future directly rather than using this constructor.
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F: Future> Future for CatchUnwindFuture<F> {
    type Output = Result<F::Output, Box<dyn std::any::Any + Send + 'static>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // SAFETY: We use `AssertUnwindSafe` here because `pin_project` guarantees
        // the pinning invariants, and we need unwind safety to call `catch_unwind`.
        //
        // Callers must ensure the wrapped future does not violate logical unwind safety
        match std::panic::catch_unwind(AssertUnwindSafe(move || this.future.poll(cx))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Err(error) => Poll::Ready(Err(error)),
        }
    }
}
