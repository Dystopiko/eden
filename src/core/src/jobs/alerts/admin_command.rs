use eden_background_worker::BackgroundJob;
use eden_gateway_api::alerts::admin_commands::{
    AdminCommandAlert as EncodedAdminCommandAlert, Executor,
};
use eden_twilight::http::ResponseFutureExt;
use erased_report::ErasedReport;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc, time::Duration};
use twilight_model::id::Id;

use crate::jobs::JobContext;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AdminCommandAlertJob(pub EncodedAdminCommandAlert);

impl BackgroundJob for AdminCommandAlertJob {
    const TYPE: &'static str = "eden::alerts::admin_command";
    const TIMEOUT: Duration = Duration::from_mins(1);

    type Context = Arc<JobContext>;

    #[tracing::instrument(skip_all, fields(
        alert.command = ?self.command,
        alert.executor = ?self.executor,
    ))]
    async fn run(&self, ctx: Self::Context) -> Result<(), ErasedReport> {
        // let content = match &self.executor {
        //     Executor::Console => Cow::Borrowed("**Someone used a privileged command!!**"),
        //     Executor::Player(info) => {
        //         Cow::Owned(format!("**{} used a privileged command!!**", info.username))
        //     }
        // };

        // let request = ctx
        //     .discord
        //     .create_message(todo!())
        //     .content(&content);

        // request.perform().await?;
        Ok(())
    }
}

impl std::ops::Deref for AdminCommandAlertJob {
    type Target = EncodedAdminCommandAlert;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
