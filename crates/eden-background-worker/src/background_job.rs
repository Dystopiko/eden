use erased_report::ErasedReport;
use error_stack::{Report, ResultExt};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

const DEFAULT_MAX_RETRIES: u16 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = 60 * 10; // 10 minutes

#[allow(async_fn_in_trait)]
pub trait BackgroundJob: DeserializeOwned + Serialize + Send + Sync + 'static {
    /// The key to use for storing this job, and looking it up later.
    ///
    /// It must be **unique** for the entire application.
    const TYPE: &'static str;

    /// Priority level of the background job.
    const PRIORITY: i16 = 0;

    /// The maximum amount of time for the job to complete before marking
    /// it as failed regardless of its result.
    ///
    /// It defaults to 10 minutes.
    const TIMEOUT: Duration = Duration::from_secs(DEFAULT_TIMEOUT_SECS);

    /// The maximum amount of retries before a task is marked as failed.
    const MAX_RETRIES: Option<u16> = Some(DEFAULT_MAX_RETRIES);

    /// When set to `false`, [`BackgroundJob::enqueue`] will check for an existing
    /// pending or running job of this type before inserting, it actually has
    /// existing pending or running job, it will attempt to not insert it.
    ///
    /// Defaults to `true` (duplicates allowed).
    const ALLOW_DUPLICATES: bool = true;

    /// The application data provided to this job at runtime.
    type Context: Send + 'static;

    /// This method defines the logic of a background job.
    ///
    /// If this function got panicked, it will not abort the entire program
    /// however, you are responsible for ensuring that any shared state accessible
    /// by the wrapped future is not left in an inconsistent state after a panic.
    ///
    /// If the future touches `Mutex`-guarded state or other shared resources,
    /// those should be reviewed for logical unwind safety.
    fn run(&self, context: Self::Context) -> impl Future<Output = Result<(), ErasedReport>> + Send;

    /// Attempts to enqueue a background job into the database to be picked
    /// up by a worker. How it will soon run depends on the queue depth and
    /// the job schema's set priority.
    ///
    /// If duplication is allowed, a new job is always inserted, otherwise if
    /// a specific job type has already inserted into the database
    /// (pending/running), nothing is returned.
    #[tracing::instrument(
        skip_all,
        name = "background_job.enqueue",
        fields(job.type = ?Self::TYPE)
    )]
    async fn enqueue(
        &self,
        conn: &mut eden_sqlite::Connection,
    ) -> Result<Option<Uuid>, Report<EnqueueJobError>> {
        let query = eden_database::BackgroundJob::new()
            .data(self)
            .change_context(EnqueueJobError::Serialize)?
            .kind(Self::TYPE)
            .priority(Self::PRIORITY)
            .build();

        let result = if Self::ALLOW_DUPLICATES {
            query.enqueue(conn).await.map(Some)
        } else {
            query.enqueue_unique(conn).await
        };
        result.change_context(EnqueueJobError::Database)
    }
}

#[derive(Debug, Error)]
pub enum EnqueueJobError {
    #[error("Failed to serialize background job")]
    Serialize,

    #[error("Could not enqueue background job to the database")]
    Database,
}
