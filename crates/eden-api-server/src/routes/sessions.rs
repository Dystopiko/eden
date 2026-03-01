use eden_api_types::common::{ApiError, ApiErrorType, MemberDiscordInfo};
use eden_api_types::sessions::{RequestSession, RequestSessionResponse};
use eden_database::DatabasePools;
use eden_database::primary_guild::{McAccount, Member};
use eden_kernel::Kernel;
use eden_sqlite::error::QueryResultExt;

use axum::Json;
use std::sync::Arc;

use crate::ApiResult;

pub async fn try_grant_session(
    kernel: Arc<Kernel>,
    Json(body): Json<RequestSession>,
) -> ApiResult<Json<RequestSessionResponse>> {
    let mut conn = kernel.db_read().await?;
    let member = if let Some(account) = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?
    {
        if account.is_bedrock() != body.bedrock {
            return Err(Box::new(ApiError {
                error: ApiErrorType::Request,
                message: "Illegal account type".into(),
            }));
        }

        let member = Member::find_by_discord_user_id(&mut conn, account.discord_user_id).await?;
        Some(MemberDiscordInfo::from_db(member))
    } else {
        None
    };

    Ok(Json(RequestSessionResponse::Granted {
        last_login_at: None,
        perks: Vec::new(),
        discord: member,
    }))
}
