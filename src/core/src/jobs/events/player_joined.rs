use eden_background_worker::BackgroundJob;
use eden_database::primary_guild::logged_in_event::NewLoggedInEvent;
use eden_twilight::http::ResponseFutureExt;
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use twilight_model::id::Id;

use crate::jobs::JobContext;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct OnPlayerJoined(pub NewLoggedInEvent);

impl BackgroundJob for OnPlayerJoined {
    const TYPE: &'static str = "eden::events::player_joined";
    const TIMEOUT: Duration = Duration::from_secs(30);

    type Context = Arc<JobContext>;

    #[tracing::instrument(skip_all)]
    async fn run(&self, ctx: Self::Context) -> Result<(), ErasedReport> {
        let mut conn = ctx.kernel.pools.db_write().await?;
        self.0.create(&mut conn).await?;
        conn.commit().await.map_err(ErasedReport::new)?;

        let request = ctx
            .discord
            .create_message(todo!())
            .content("Someone joined the server!");

        request.perform().await?;
        Ok(())
    }
}
