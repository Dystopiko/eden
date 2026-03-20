use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_core::jobs::OnPlayerJoined;
use eden_database::primary_guild::{LoggedInEvent, McAccount, McAccountType, Member};
use eden_gateway_api::{
    member::EncodedMember,
    sessions::request::{RequestSession, SessionGranted},
};
use eden_sqlite::error::QueryResultExt;
use std::sync::Arc;

use crate::{
    controllers::ApiResult,
    errors::{ApiError, ErrorCode},
    extract::{Kernel, Validated},
    middleware::trace_request::RequestLogs,
    ratelimiter::{Actor, LimitedAction, RateLimiter},
};

pub async fn post(
    Kernel(kernel): Kernel,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    Extension(logs): Extension<RequestLogs>,
    Validated(body): Validated<Json<RequestSession>>,
) -> ApiResult<Response> {
    let mut encoded_member = None::<EncodedMember>;
    let mut actor = Actor::Ip(body.ip);
    let mut account_type = if body.java {
        McAccountType::Java
    } else {
        McAccountType::Bedrock
    };

    if let Some((member, account)) = find_mc_account(&kernel, &body, &logs).await? {
        actor = Actor::Member(member.discord_user_id.cast());
        account_type = account.kind;
        encoded_member = Some(EncodedMember::from_db(member));
    }

    let action = LimitedAction::RequestSession {
        guest: encoded_member.is_none(),
    };

    // Make sure player is not logging in too frequently by determining
    // their IP address or the player's discord ID.
    logs.add("ratelimit.action", format!("{action:?}"));
    logs.add("ratelimit.actor", format!("{actor:?}"));

    let rl_headers = rate_limiter.permit(actor, action)?.into_headers();

    // Log the granted session
    let new_event = LoggedInEvent::new_event()
        .ip_address(body.ip)
        .kind(account_type)
        .maybe_member_id(encoded_member.as_ref().map(|v| v.id))
        .maybe_username(encoded_member.as_ref().map(|v| v.name.to_string()))
        .player_uuid(body.uuid)
        .build();

    // Session granted!
    let body = SessionGranted {
        last_login_at: None,
        member: encoded_member,
        perks: Vec::new(),
    };

    kernel.enqueue_job(OnPlayerJoined(new_event)).await?;
    Ok((StatusCode::CREATED, rl_headers, Json(body)).into_response())
}

async fn find_mc_account(
    kernel: &eden_core::Kernel,
    body: &RequestSession,
    logs: &RequestLogs,
) -> ApiResult<Option<(Member, McAccount)>> {
    let mut conn = kernel.pools.db_read_prefer_primary().await?;
    let account = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?;

    logs.add("account.exists", account.is_some());
    let Some(account) = account else {
        return Ok(None);
    };

    logs.add("account.java", account.is_java());
    if account.is_bedrock() == body.java {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "Incompatible account type",
        ));
    }

    let member = Member::find_by_discord_user_id(&mut conn, account.discord_user_id).await?;
    Ok(Some((member, account)))
}
