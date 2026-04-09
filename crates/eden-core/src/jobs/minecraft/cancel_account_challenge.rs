use eden_background_worker::BackgroundJob;
use eden_database::primary_guild::McAccountChallenge;
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

use crate::jobs::JobContext;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CancelMcAccountChallenge(pub Uuid);

impl BackgroundJob for CancelMcAccountChallenge {
    const MAX_RETRIES: Option<u16> = None;
    const PRIORITY: i16 = 100;
    const TYPE: &'static str = "eden::members::cancel_account_challenge";
    const TIMEOUT: Duration = Duration::from_secs(20);

    type Context = Arc<JobContext>;

    #[tracing::instrument(skip_all, fields(challenge.id = %self.0))]
    async fn run(&self, ctx: Self::Context) -> Result<(), ErasedReport> {
        let mut conn = ctx.kernel.pools.db_write().await?;
        let challenge_id = self.0;

        McAccountChallenge::mark_cancelled(&mut conn, challenge_id).await?;
        conn.commit().await.map_err(ErasedReport::new)
    }
}
