use serde::Deserialize;

/// Configuration for background jobs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct BackgroundJobs {
    pub enabled: bool,
    pub workers: usize,
}

impl Default for BackgroundJobs {
    fn default() -> Self {
        Self {
            enabled: true,
            workers: num_cpus::get().max(4),
        }
    }
}
