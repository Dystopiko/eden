use eden_database::{
    DatabasePools, Timestamp,
    primary_guild::{McAccount, Member},
};
use eden_sqlite::error::QueryResultExt;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use axum::{
    Extension,
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    controllers::Kernel,
    middleware::trace_request::RequestLogs,
    model::MemberView,
    result::{ApiError, ApiResult, ErrorCode},
};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestSession {
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

pub async fn try_grant(
    Kernel(kernel): Kernel,
    Extension(logs): Extension<RequestLogs>,
    Json(body): Json<RequestSession>,
) -> ApiResult<Response> {
    let mut conn = kernel.db_read().await?;
    let account = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?;

    let mut member = None;
    logs.add("account.exists", account.is_some());

    if let Some(account) = account {
        logs.add("account.kind", account.uuid);
        member = Some(acquire_member(&mut conn, &account, &body).await?);
    }

    let payload = Json(SessionGranted {
        last_login_at: None,
        perks: Vec::new(),
        member,
    });

    Ok((StatusCode::CREATED, payload).into_response())
}

async fn acquire_member(
    conn: &mut eden_sqlite::Connection,
    account: &McAccount,
    body: &RequestSession,
) -> ApiResult<MemberView> {
    let member = Member::find_by_discord_user_id(conn, account.discord_user_id).await?;
    if account.is_bedrock() != body.bedrock {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "Incompatible account type",
        ))?;
    }

    Ok(member.into())
}
