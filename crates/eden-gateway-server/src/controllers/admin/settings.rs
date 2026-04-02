use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_database::settings::UpsertSettings;
use eden_gateway_api::{admin::settings::PatchSettings, settings::EncodedSettings};
use erased_report::ErasedReport;

use crate::{controllers::ApiResult, extract::Kernel};

pub async fn get(Kernel(kernel): Kernel) -> ApiResult<Response> {
    let current = kernel.settings().await?;
    let encoded: EncodedSettings = current.into();
    Ok(Json(encoded).into_response())
}

pub async fn patch(Kernel(kernel): Kernel, Json(body): Json<PatchSettings>) -> ApiResult<Response> {
    let current = kernel.settings().await?;
    let guild_id = kernel.config.bot.primary_guild.id;

    let mut conn = kernel.pools.db_write().await?;
    let query = UpsertSettings::builder()
        .guild_id(guild_id.into())
        .allow_guests(body.allow_guests.unwrap_or(current.allow_guests))
        .build();

    query.perform(&mut conn).await?;
    conn.commit().await.map_err(ErasedReport::new)?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
