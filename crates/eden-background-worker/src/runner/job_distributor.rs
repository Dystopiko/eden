use eden_database::{BackgroundJob, DatabasePools};
use eden_utils::signals::ShutdownSignal;
use erased_report::ErasedReport;
use std::time::Duration;
use tokio::time::MissedTickBehavior;

pub struct JobDistributor {
    pub pools: DatabasePools,
    pub poll_interval: Duration,
    pub shutdown_signal: ShutdownSignal,
    pub tx: async_channel::Sender<BackgroundJob>,
}

impl JobDistributor {
    pub async fn run(&self) {
        tracing::debug!("started job distributor");

        let mut interval = tokio::time::interval(self.poll_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            // fetch_next_job is not cancel safe, so we need to wait for it to
            // be completed before we can attempt to shut down
            if self.shutdown_signal.initiated() {
                tracing::debug!("job distributor terminated");
                break;
            }

            if self.tx.is_full() {
                tokio::select! {
                    _ = self.shutdown_signal.subscribe() => {},
                    _ = interval.tick() => {},
                }
                continue;
            }

            match self.fetch_next_job().await {
                Ok(Some(job)) => {
                    _ = self.tx.send(job).await;
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(?error, "error occurred while trying to fetch next job");
                }
            };
        }
    }

    async fn fetch_next_job(&self) -> Result<Option<BackgroundJob>, ErasedReport> {
        let mut conn = self.pools.db_write().await?;
        let job = BackgroundJob::pull_next_pending(&mut conn, None).await?;
        conn.commit().await.map_err(ErasedReport::new)?;
        Ok(job)
    }
}
