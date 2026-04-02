use axum::{Json, extract::Path};
use eden_database::primary_guild::Member;
use eden_gateway_api::admin::members::Invitees;
use futures::TryStreamExt;
use twilight_model::id::{Id, marker::UserMarker};

use crate::{controllers::ApiResult, extract::Kernel};

pub async fn invitees(
    Kernel(kernel): Kernel,
    Path(member_id): Path<Id<UserMarker>>,
) -> ApiResult<Json<Invitees>> {
    let mut conn = kernel.pools.db_read().await?;

    let mut stream = Member::fetch_invitees(&mut conn, member_id);
    let mut invitees = Vec::new();

    while let Some(member) = stream.try_next().await? {
        invitees.push(member.into());
    }

    Ok(Json(Invitees {
        count: invitees.len().try_into().unwrap_or(0),
        invitees,
    }))
}
