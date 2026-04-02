use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_core::jobs::admin_command::AdminCommandAlertJob;
use eden_gateway_api::alerts::admin_commands::AdminCommandAlert as Form;

use crate::{
    controllers::ApiResult,
    extract::{Kernel, Validated},
};

pub async fn publish(
    Kernel(kernel): Kernel,
    Validated(Json(form)): Validated<Json<Form>>,
) -> ApiResult<Response> {
    kernel.enqueue_job(AdminCommandAlertJob(form)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
