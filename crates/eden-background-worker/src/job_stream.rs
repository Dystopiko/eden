use async_trait::async_trait;
use eden_database::BackgroundJob;
use erased_report::ErasedReport;

/// Abstracts over the source of pending background jobs.
#[async_trait]
pub trait JobStream: Send + Sync + 'static {
    /// Fetch the next pending job from this provider's queue.
    async fn fetch_next_job(&mut self) -> Result<Option<BackgroundJob>, ErasedReport>;
}

#[async_trait]
impl JobStream for eden_database::DatabasePools {
    async fn fetch_next_job(&mut self) -> Result<Option<BackgroundJob>, ErasedReport> {
        let mut conn = self.db_write().await?;
        let Some(job) = BackgroundJob::pull_next_pending(&mut conn, None).await? else {
            return Ok(None);
        };

        conn.commit().await.map_err(ErasedReport::new)?;
        Ok(Some(job))
    }
}
