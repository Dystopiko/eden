use axum::{
    extract::{Extension, Json},
    response::{IntoResponse, Response},
};
use eden_database::views::McAccountView;
use eden_gateway_api::sessions::validate::{PlayerEntry, ValidatePlayers, ValidatePlayersResponse};
use eden_sqlite::error::QueryResultExt;
use std::{collections::HashMap, sync::Arc};

use crate::{
    controllers::ApiResult,
    extract::{Kernel, Validated},
    ratelimiter::{Actor, LimitedAction, RateLimiter},
};

pub async fn validate(
    Kernel(kernel): Kernel,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    Validated(body): Validated<Json<ValidatePlayers>>,
) -> ApiResult<Response> {
    let headers = rate_limiter
        .permit(Actor::McServer, LimitedAction::ValidateSessions)?
        .into_headers();

    let mut conn = kernel.pools.db_read_prefer_primary().await?;
    let mut players = HashMap::new();
    for &id in body.players.iter() {
        let account = McAccountView::find_by_mc_uuid(&mut conn, id)
            .await
            .optional()?;

        let Some(account) = account else {
            players.insert(id, None);
            continue;
        };

        let perks = kernel.resolve_mc_perks(&account);
        let entry = PlayerEntry {
            member: account.into(),
            perks,
        };

        players.insert(id, Some(entry));
    }

    Ok((headers, Json(ValidatePlayersResponse { players })).into_response())
}
