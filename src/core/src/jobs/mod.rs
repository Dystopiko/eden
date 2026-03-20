use bon::Builder;
use eden_background_worker::Runner;
use std::sync::Arc;

pub mod alerts;
pub mod events;

pub use self::alerts::*;
pub use self::events::*;

#[derive(Debug, Builder)]
#[builder(finish_fn(name = "build_inner", vis = ""))]
pub struct JobContext {
    /// Discord Rest API client
    pub discord: Arc<twilight_http::Client>,

    /// The current kernel of the session.
    pub kernel: Arc<crate::Kernel>,
}

impl<S: job_context_builder::State> JobContextBuilder<S> {
    /// Creates a new [`JobContext`] and wraps it in an [`Arc`] for shared ownership.
    #[must_use]
    pub fn build(self) -> Arc<JobContext>
    where
        S: job_context_builder::IsComplete,
    {
        Arc::new(self.build_inner())
    }
}

pub trait RunnerExt {
    fn register_core_job_types(self) -> Self;
}

impl RunnerExt for Runner<Arc<JobContext>> {
    fn register_core_job_types(self) -> Self {
        self.register_job_type::<AdminCommandAlertJob>()
            .register_job_type::<OnPlayerJoined>()
    }
}
