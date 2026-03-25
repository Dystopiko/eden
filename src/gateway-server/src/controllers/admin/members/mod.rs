use axum::{
    Extension, Json,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_database::{
    Timestamp,
    primary_guild::{McAccount, Member},
    views::MemberView,
};
use eden_gateway_api::{
    admin::members::PatchMember,
    members::{EncodedMember, FullMember},
};
use eden_sqlite::error::QueryResultExt;
use eden_twilight::http::{HttpFailReason, HttpResultExt, ResponseFutureExt};
use erased_report::ErasedReport;
use std::sync::Arc;
use twilight_model::id::{Id, marker::UserMarker};

use crate::{
    controllers::ApiResult,
    errors::{ApiError, ErrorCode},
    extract::Kernel,
    middleware::extract_client_ip::ClientIp,
    ratelimiter::RateLimiter,
    ratelimiter::{Actor, LimitedAction},
};

pub mod invitees;

// Retrieved from: https://discord.com/developers/docs/topics/opcodes-and-status-codes#http
const UNKNOWN_GUILD: u64 = 10004;
const UNKNOWN_MEMBER: u64 = 10007;

pub async fn post(
    Kernel(kernel): Kernel,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    Path(member_id): Path<Id<UserMarker>>,
) -> ApiResult<Response> {
    let rl_headers = rate_limiter
        .permit(Actor::Ip(client_ip), LimitedAction::RegisterMember)?
        .into_headers();

    let result = kernel
        .http
        .guild_member(kernel.config.bot.primary_guild.id, member_id)
        .perform()
        .await;

    let member = match result {
        Ok(res) => res.model().await.simplify_error()?,
        Err(error) => match error.current_context().reason() {
            HttpFailReason::Response(UNKNOWN_GUILD) => {
                return Err(ApiError::from_static(
                    ErrorCode::NotFound,
                    "The bot may not exist in the primary guild",
                ));
            }
            HttpFailReason::Response(UNKNOWN_MEMBER) => {
                return Err(ApiError::from_static(
                    ErrorCode::NotFound,
                    "Member may not exist in the primary guild",
                ));
            }
            _ => return Err(error.into()),
        },
    };

    if member.user.bot {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "Bots are not allowed to be a member",
        ));
    }

    let joined_at = member
        .joined_at
        .map(|v| Timestamp::from_secs(v.as_secs()).unwrap());

    let mut conn = kernel.pools.db_write().await?;
    Member::upsert()
        .name(&member.user.name)
        .discord_user_id(member_id)
        .maybe_joined_at(joined_at)
        .build()
        .perform(&mut conn)
        .await?;

    let view = MemberView::find_by_discord_user_id(&mut conn, member_id).await?;
    let body: Json<EncodedMember> = Json(view.into());

    conn.commit().await.map_err(ErasedReport::new)?;
    Ok((StatusCode::CREATED, rl_headers, body).into_response())
}

pub async fn patch(
    Kernel(kernel): Kernel,
    Path(member_id): Path<Id<UserMarker>>,
    Json(body): Json<PatchMember>,
) -> ApiResult<Response> {
    let mut conn = kernel.pools.db_write().await?;

    // Make sure that member exists in invited_by
    if let Some(invited_by) = body.invited_by {
        let exists = Member::find_by_discord_user_id(&mut conn, invited_by)
            .await
            .optional()?
            .is_some();

        if !exists {
            return Err(ApiError::from_static(
                ErrorCode::InvalidRequest,
                "Inviter must be a member",
            ));
        }
    }

    // This is to invalidate the request if we cannot find a specific member
    let member = MemberView::find_by_discord_user_id(&mut conn, member_id).await?;
    Member::upsert()
        .discord_user_id(member_id)
        .maybe_invited_by(body.invited_by)
        .name(&body.name.unwrap_or(member.name))
        .build()
        .perform(&mut conn)
        .await?;

    conn.commit().await.map_err(ErasedReport::new)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn get(
    Kernel(kernel): Kernel,
    Path(member_id): Path<Id<UserMarker>>,
) -> ApiResult<Json<FullMember>> {
    let mut conn = kernel.pools.db_read().await?;

    let view = MemberView::find_by_discord_user_id(&mut conn, member_id).await?;
    let accounts = McAccount::get_all(&mut conn, member_id).await?;

    Ok(Json((view, accounts).into()))
}
