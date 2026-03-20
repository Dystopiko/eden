use bon::Builder;
use eden_database::{BackgroundJob, DatabasePools};
use eden_utils::{futures::CatchUnwind, signals::ShutdownSignal};
use erased_report::ErasedReport;
use std::{panic::PanicHookInfo, sync::Arc, time::Duration};
use thiserror::Error;
use tracing::Instrument;
use uuid::Uuid;

use crate::{
    job_stream::JobStream,
    registry::{JobRegistry, RegistryItem},
};

#[derive(Builder)]
pub struct Worker<Context: Clone + Send + Sync + 'static> {
    context: Context,
    pools: DatabasePools,
    poll_interval: Duration,
    registry: Arc<JobRegistry<Context>>,
    shutdown_signal: ShutdownSignal,
    stream: Box<dyn JobStream>,
}

impl<Context> Worker<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    pub async fn run(&mut self) {
        loop {
            // Wait for a background job to be completed before shutting this down
            if self.shutdown_signal.initiated() {
                break;
            }

            match self.run_next_job().await {
                Ok(Some(..)) => continue,
                Ok(None) => {
                    tracing::trace!("no pending background worker jobs found");
                }
                Err(error) => {
                    tracing::debug!(?error, "failed to run next job");
                }
            };

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn run_next_job(&mut self) -> Result<Option<Uuid>, ErasedReport> {
        let Some(job) = self.stream.fetch_next_job().await? else {
            return Ok(None);
        };

        let span = tracing::info_span!(
            "worker.run_next_job",
            job.id = %job.id,
            job.created_at = ?job.created_at,
            job.kind = ?job.kind,
            job.data.len = job.data.len(),
            job.last_retry = ?job.last_retry,
            job.priority = %job.priority,
            job.retries = %job.retries,
        );

        let Some(item) = self.registry.item(&job.kind) else {
            span.in_scope(|| tracing::warn!("unknown job type {:?}", job.kind));
            return Ok(None);
        };

        if !span.is_disabled() {
            span.record("job.max_retries", tracing::field::debug(item.max_retries));
            span.record("job.timeout", tracing::field::debug(item.timeout));
        }

        span.in_scope(|| tracing::debug!("found background job; running..."));

        let future = (*item.run)(self.context.clone(), job.data).catch_unwind();
        let result = tokio::time::timeout(item.timeout, future)
            .await
            .map_err(|_| ErasedReport::new(JobTimedOut))
            .and_then(|res| res.map_err(make_panic_report))
            .flatten();

        let mut conn = self.pools.db_write().await?;
        self.handle_job_result(&mut conn, job.id, item, result)
            .instrument(span)
            .await?;

        conn.commit().await.map_err(ErasedReport::new)?;
        Ok(Some(job.id))
    }

    async fn handle_job_result(
        &self,
        conn: &mut eden_sqlite::Transaction<'static>,
        job_id: Uuid,
        registry: &RegistryItem<Context>,
        result: Result<(), ErasedReport>,
    ) -> Result<(), ErasedReport> {
        let Err(error) = result else {
            tracing::debug!("deleting successful job");
            BackgroundJob::delete(conn, job_id).await?;
            return Ok(());
        };

        if error.contains::<JobTimedOut>() {
            tracing::warn!(?error, "job got timed out");
        } else {
            tracing::warn!(?error, "failed to run job");
        }

        let status = BackgroundJob::requeue_or_fail(conn, job_id, registry.max_retries).await?;
        if status == eden_database::JobStatus::Failed {
            tracing::warn!(?error, "max tries exceeded; aborting background job");
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("background job timed out")]
struct JobTimedOut;

#[derive(Debug, Error)]
#[error("background job panicked")]
struct JobPanicked;

fn make_panic_report(payload: Box<dyn std::any::Any + Send + 'static>) -> ErasedReport {
    let cause = payload
        .downcast_ref::<PanicHookInfo<'_>>()
        .map(ToString::to_string)
        .or_else(|| {
            let cause = payload.downcast_ref::<&'static str>();
            cause.map(ToString::to_string)
        })
        .or_else(|| payload.downcast_ref::<String>().map(String::to_string))
        .unwrap_or_else(|| "<unknown>".into());

    ErasedReport::new(JobPanicked).attach(format!("panic cause: {cause}"))
}
