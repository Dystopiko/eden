use async_trait::async_trait;
use eden_database::DatabasePools;
use eden_utils::signals::ShutdownSignal;
use erased_report::ErasedReport;
use futures::future::join_all;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::{BackgroundJob, JobRegistry, job_stream::JobStream, worker::Worker};

mod job_distributor;
use self::job_distributor::JobDistributor;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_DISTRIBUTOR_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_WORKERS: usize = 1;

#[must_use = "runners do not do anything unless you run them"]
pub struct Runner<Context> {
    context: Context,
    poll_interval: Duration,
    pools: DatabasePools,
    registry: JobRegistry<Context>,
    workers: usize,
}

impl<Context> Runner<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    pub fn new(context: Context, pools: DatabasePools) -> Self {
        Self {
            context,
            poll_interval: DEFAULT_POLL_INTERVAL,
            pools,
            registry: JobRegistry::empty(),
            workers: DEFAULT_WORKERS,
        }
    }

    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn register_job_type<T: BackgroundJob<Context = Context>>(mut self) -> Self {
        self.registry.register_job_type::<T>();
        self
    }

    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }
}

impl<Context> Runner<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    pub fn start(self) -> RunHandle {
        tracing::debug!("launching {} background job worker(s)", self.workers);

        let mut handles = Vec::with_capacity(self.workers);
        let registry = Arc::new(self.registry);
        let shutdown_signal = ShutdownSignal::new();

        for id in 0..self.workers {
            let name = format!("background-worker-{id}");
            let span = tracing::info_span!("worker", worker.name = ?name);
            tracing::info!(worker.name = ?name, "starting worker");

            let mut worker = Worker::<Context>::builder()
                .context(self.context.clone())
                .poll_interval(self.poll_interval)
                .pools(self.pools.clone())
                .registry(registry.clone())
                .shutdown_signal(shutdown_signal.clone())
                .stream(Box::new(self.pools.clone()) as _)
                .build();

            let handle = tokio::spawn(async move { worker.run().instrument(span).await });
            handles.push(handle);
        }

        RunHandle {
            handles,
            shutdown_signal,
        }
    }

    pub fn start_with_job_distributor(self, poll_interval: Option<Duration>) -> RunHandle {
        tracing::debug!(
            "launching {} background job worker(s) with job distributor",
            self.workers
        );

        let mut handles = Vec::with_capacity(self.workers);
        let poll_interval = poll_interval.unwrap_or(DEFAULT_DISTRIBUTOR_POLL_INTERVAL);
        let registry = Arc::new(self.registry);
        let shutdown_signal = ShutdownSignal::new();

        let (tx, rx) = async_channel::bounded::<eden_database::BackgroundJob>(self.workers);
        for id in 0..self.workers {
            let name = format!("background-worker-{id}");
            let span = tracing::info_span!("worker", worker.name = ?name);
            tracing::info!(worker.name = ?name, "starting worker");

            let mut worker = Worker::<Context>::builder()
                .context(self.context.clone())
                .poll_interval(self.poll_interval)
                .pools(self.pools.clone())
                .registry(registry.clone())
                .shutdown_signal(shutdown_signal.clone())
                .stream(Box::new(ReceiverChannel { rx: rx.clone() }) as _)
                .build();

            let handle = tokio::spawn(async move { worker.run().instrument(span).await });
            handles.push(handle);
        }

        let distributor = JobDistributor {
            pools: self.pools,
            poll_interval,
            shutdown_signal: shutdown_signal.clone(),
            tx,
        };
        tokio::spawn(async move { distributor.run().await });

        RunHandle {
            handles,
            shutdown_signal,
        }
    }
}

pub struct RunHandle {
    handles: Vec<JoinHandle<()>>,
    shutdown_signal: ShutdownSignal,
}

impl RunHandle {
    pub async fn shutdown(self) {
        tracing::info!("shutting down {} background worker(s)", self.handles.len());

        self.shutdown_signal.initiate();
        for result in join_all(self.handles).await {
            if let Err(error) = result {
                tracing::warn!(?error, "background worker task panicked");
            }
        }
    }
}

struct ReceiverChannel {
    rx: async_channel::Receiver<eden_database::BackgroundJob>,
}

#[async_trait]
impl JobStream for ReceiverChannel {
    async fn fetch_next_job(
        &mut self,
    ) -> Result<Option<eden_database::BackgroundJob>, ErasedReport> {
        match self.rx.recv().await {
            Ok(job) => Ok(Some(job)),
            Err(..) => {
                // Maybe the distributor has shut down or panicked. Worker loop will
                // have to wait for the shutdown and exit cleanly.
                Ok(None)
            }
        }
    }
}
