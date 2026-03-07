use bon::Builder;
use eden_config::Config;
use eden_kernel::Kernel;
use splinter::ShardHandle;
use std::sync::Arc;
use twilight_standby::Standby;

#[derive(Debug, Builder)]
pub struct EventContext {
    pub kernel: Arc<Kernel>,
    pub shard: ShardHandle,

    /// Retained for future use by triggers/commands that need to wait for a
    /// specific follow-up event (e.g. a button interaction or a message reply).
    #[allow(dead_code)]
    pub standby: Arc<Standby>,
}

impl EventContext {
    /// Convenience function for accessing [`Config`].
    ///
    /// [`Config`]: eden_config::Config
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.kernel.config
    }
}
