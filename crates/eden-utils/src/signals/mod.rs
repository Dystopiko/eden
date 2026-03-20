use futures::future::Either::*;
use std::fmt;
use tokio::sync::watch;

#[derive(Clone)]
pub struct ShutdownSignal {
    tx: watch::Sender<bool>,
}

impl ShutdownSignal {
    /// Creates a new, non-initiated shutdown signal.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (tx, _) = watch::channel(false);
        Self { tx }
    }

    /// Runs a `Result`-returning future while cooperatively listening for shutdown.
    ///
    /// This method combines [`run_or_cancelled`](Self::run_or_cancelled) with
    /// ergonomic handling of `Result<T, E>` futures.
    ///
    /// It returns:
    ///
    /// - `Ok(Some(value))` if the future completed successfully before shutdown.
    /// - `Err(error)` if the future completed with an error before shutdown.
    /// - `Ok(None)` if shutdown was initiated before the future completed.
    ///
    /// # Cancel safety
    ///
    /// This method may be cancel safe depending on the future it is
    /// being used to run. If the future is not cancel safe, it is not
    /// advisable to use this function.
    pub async fn run_result_or_cancelled<T, E, F: Future<Output = Result<T, E>>>(
        &self,
        future: F,
    ) -> Result<Option<T>, E> {
        match self.run_or_cancelled(future).await {
            Some(Ok(okay)) => Ok(Some(okay)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    /// Runs a future until either:
    ///
    /// - The future completes successfully, returning `Some(output)`, or
    /// - Shutdown is initiated, returning `None`.
    ///
    /// # Cancel safety
    ///
    /// This method may be cancel safe depending on the future it is
    /// being used to run. If the future is not cancel safe, it is not
    /// advisable to use this function.
    #[must_use]
    pub async fn run_or_cancelled<F: Future>(&self, future: F) -> Option<F::Output> {
        let shutdown = Box::pin(self.subscribe());
        let future = Box::pin(future);
        match futures::future::select(shutdown, future).await {
            Left((..)) => None,
            Right((output, ..)) => Some(output),
        }
    }

    /// Waits asynchronously until shutdown is initiated.
    ///
    /// If shutdown has already been initiated, this method
    /// returns immediately.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe.
    pub async fn subscribe(&self) {
        let mut rx = self.tx.subscribe();
        if *rx.borrow() {
            return;
        }
        _ = rx.wait_for(|initiated| *initiated).await;
    }

    /// Initiates shutdown.
    ///
    /// # Idempotency
    ///
    /// Calling this method multiple times is safe.
    /// Subsequent calls have no additional effect.
    pub fn initiate(&self) {
        let _ = self.tx.send_replace(true);
    }

    /// Returns `true` if shutdown has been initiated.
    ///
    /// This is a fast, non-blocking state check.
    #[must_use]
    pub fn initiated(&self) -> bool {
        *self.tx.borrow()
    }
}

impl fmt::Debug for ShutdownSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShutdownSignal")
            .field("initiated", &self.initiated())
            .finish()
    }
}

impl PartialEq for ShutdownSignal {
    fn eq(&self, other: &Self) -> bool {
        self.tx.same_channel(&other.tx)
    }
}

impl Eq for ShutdownSignal {}

#[cfg(test)]
mod tests {
    use tokio::task::JoinSet;
    use tokio::time::timeout;

    use crate::signals::ShutdownSignal;
    use std::time::Duration;

    #[tokio::test]
    async fn should_receive_shutdown() {
        let signal = ShutdownSignal::new();
        let child = signal.clone();

        let handle = tokio::spawn(async move {
            child.subscribe().await;
        });

        signal.initiate();
        let result = timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "waiting task did not receive shutdown");
    }

    #[tokio::test]
    async fn initiate_should_be_idempotent() {
        let signal = ShutdownSignal::new();
        signal.initiate();
        signal.initiate();
        signal.initiate();

        let result = timeout(Duration::from_millis(10), signal.subscribe()).await;
        assert!(result.is_ok());
        assert!(signal.initiated());
    }

    #[tokio::test]
    async fn should_not_deadlock() {
        let mut tasks = JoinSet::new();
        let signal = ShutdownSignal::new();

        for _ in 0..10_000 {
            let child = signal.clone();
            tasks.spawn(async move {
                child.subscribe().await;
            });
        }

        signal.initiate();

        let result = timeout(Duration::from_secs(1), async {
            while tasks.join_next().await.is_some() {}
        })
        .await;

        assert!(result.is_ok(), "tasks did not shut down in time");
    }

    #[tokio::test]
    async fn should_notify_waiters() {
        let mut tasks = JoinSet::new();
        let signal = ShutdownSignal::new();

        for _ in 0..10 {
            let child = signal.clone();
            tasks.spawn(async move {
                child.subscribe().await;
            });
        }

        signal.initiate();

        let result = timeout(Duration::from_secs(1), async {
            while tasks.join_next().await.is_some() {}
        })
        .await;

        assert!(result.is_ok(), "tasks did not shut down in time");
    }

    #[tokio::test]
    async fn late_subscriber_should_return_immediately() {
        use tokio::time::{Duration, timeout};

        let signal = ShutdownSignal::new();
        signal.initiate();

        let result = timeout(Duration::from_millis(10), signal.subscribe()).await;
        assert!(result.is_ok(), "late subscriber blocked");
    }
}
