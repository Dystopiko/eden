use eden_database::{
    DatabasePools, Timestamp,
    primary_guild::{McAccount, Member},
};
use eden_sqlite::error::QueryResultExt;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    controllers::Kernel,
    result::{ApiError, ApiErrorCode, ApiResult},
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RequestSession {
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub bedrock: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionGranted {
    last_login_at: Option<Timestamp>,
    perks: Vec<String>,
    member: bool,
}

pub async fn try_grant(
    Kernel(kernel): Kernel,
    Json(body): Json<RequestSession>,
) -> ApiResult<Response> {
    let mut conn = kernel.db_read().await?;

    let account = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?;

    let mut has_member_data = false;
    if let Some(account) = account {
        validate_mc_account_with_body(&account, &body)?;

        _ = Member::find_by_discord_user_id(&mut conn, account.discord_user_id).await?;
        has_member_data = true;
    }

    let payload = Json(SessionGranted {
        last_login_at: None,
        perks: Vec::new(),
        member: has_member_data,
    });

    Ok((StatusCode::CREATED, payload).into_response())
}

fn validate_mc_account_with_body(account: &McAccount, body: &RequestSession) -> ApiResult<()> {
    if account.is_bedrock() != body.bedrock {
        return Err(ApiError::from_static(
            ApiErrorCode::InvalidRequest,
            "Incompatible account type",
        )
        .into());
    }

    Ok(())
}
