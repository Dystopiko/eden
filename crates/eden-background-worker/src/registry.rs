use erased_report::ErasedReport;
use std::{collections::HashMap, fmt, pin::Pin, sync::Arc, time::Duration};

use crate::background_job::BackgroundJob;

#[must_use]
#[derive(Clone)]
pub struct JobRegistry<Context> {
    // Key: BackgroundJob::TYPE
    entries: HashMap<String, RegistryItem<Context>>,
}

impl<Context> JobRegistry<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn register_job_type<J: BackgroundJob<Context = Context>>(&mut self)
    where
        Context: Send + 'static,
    {
        let item = RegistryItem::new::<J>();
        self.entries.insert(J::TYPE.into(), item);
    }

    #[must_use]
    pub fn is_registered<J: BackgroundJob<Context = Context>>(&self) -> bool {
        self.entries.contains_key(J::TYPE)
    }

    #[must_use]
    pub fn item(&self, kind: &str) -> Option<&RegistryItem<Context>> {
        self.entries.get(kind)
    }
}

impl<Context> Default for JobRegistry<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<Context> fmt::Debug for JobRegistry<Context> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Types<'a, Context>(
            std::collections::hash_map::Keys<'a, String, RegistryItem<Context>>,
        );

        impl<'a, C> fmt::Debug for Types<'a, C> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut f = f.debug_list();
                for key in self.0.clone() {
                    f.entry(key);
                }
                f.finish()
            }
        }

        f.debug_struct("JobRegistry")
            .field("registered_job_types", &Types(self.entries.keys()))
            .finish()
    }
}

#[derive(Clone)]
pub struct RegistryItem<Context> {
    pub max_retries: Option<u16>,
    pub run: Arc<RunJobFn<Context>>,
    pub timeout: Duration,
}

impl<Context> RegistryItem<Context>
where
    Context: Clone + Send + Sync + 'static,
{
    fn new<J: BackgroundJob<Context = Context>>() -> Self {
        let run: Arc<RunJobFn<Context>> = Arc::new(|context, data| {
            Box::pin(async move {
                let job: J = serde_json::from_str(&data).map_err(ErasedReport::new)?;
                job.run(context).await
            })
        });

        Self {
            max_retries: J::MAX_RETRIES,
            run,
            timeout: J::TIMEOUT,
        }
    }
}

type RunJobFn<Context> = dyn Fn(Context, String) -> RunJobFuture + Send + Sync;
type RunJobFuture = Pin<Box<dyn Future<Output = Result<(), ErasedReport>> + Send>>;

#[cfg(test)]
#[allow(unused)]
mod tests {
    use crate::registry::JobRegistry;

    fn requires_sync<T: Sync>() {}
    fn should_implement_sync() {
        requires_sync::<JobRegistry<()>>();
    }
}
