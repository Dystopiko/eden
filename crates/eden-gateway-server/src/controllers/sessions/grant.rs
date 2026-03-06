use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_database::{
    DatabasePools, Timestamp,
    primary_guild::{McAccount, Member},
};
use eden_sqlite::error::QueryResultExt;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use crate::{
    controllers::{ApiResult, Kernel},
    errors::{ApiError, ErrorCode},
    middleware::trace_request::RequestLogs,
    model::MemberView,
};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GrantRequest {
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub bedrock: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SessionGranted {
    last_login_at: Option<Timestamp>,
    perks: Vec<String>,
    member: Option<MemberView>,
}

pub async fn grant(
    Kernel(kernel): Kernel,
    Extension(logs): Extension<RequestLogs>,
    Json(body): Json<GrantRequest>,
) -> ApiResult<Response> {
    // Logic Flow:
    // 1. Make sure player is not logging in too frequently by determining
    //    their IP address or the player's discord ID.
    // 2. Log the granted session to the admin portion
    let mut response = SessionGranted {
        last_login_at: None,
        perks: Vec::new(),
        member: None,
    };

    if let Some((_account, member)) = find_member_combo(&kernel, &logs, &body).await? {
        response.member = Some(member.into());
    }

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

async fn find_member_combo(
    kernel: &eden_kernel::Kernel,
    logs: &RequestLogs,
    body: &GrantRequest,
) -> ApiResult<Option<(McAccount, Member)>> {
    let mut conn = kernel.db_read_prefer_primary().await?;
    let account = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?;

    logs.add("account.exists", account.is_some());
    let Some(account) = account else {
        return Ok(None);
    };

    logs.add("account.bedrock", account.is_bedrock());
    if account.is_bedrock() != body.bedrock {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "Incompatible account type",
        ));
    }

    let member = Member::find_by_discord_user_id(&mut conn, account.discord_user_id).await?;
    Ok(Some((account, member)))
}
