pub mod grant;

// use eden_database::{
//     DatabasePools, Timestamp,
//     primary_guild::{McAccount, Member},
// };
// use eden_sqlite::error::QueryResultExt;
// use serde::{Deserialize, Serialize};
// use std::net::IpAddr;
// use uuid::Uuid;

// use axum::{
//     Extension,
//     extract::Json,
//     http::StatusCode,
//     response::{IntoResponse, Response},
// };

// use crate::{
//     controllers::{ApiResult, Kernel},
//     errors::{ApiError, ErrorCode},
//     middleware::trace_request::RequestLogs,
//     model::MemberView,
// };

// pub async fn try_grant(
//     Kernel(kernel): Kernel,
//     Extension(logs): Extension<RequestLogs>,
//     Json(body): Json<RequestSession>,
// ) -> ApiResult<Response> {
//     let mut conn = kernel.db_read().await?;
//     let account = McAccount::find_by_uuid(&mut conn, body.uuid)
//         .await
//         .optional()?;

//     logs.add("account.exists", account.is_some());

//     let member = match account {
//         None => None,
//         Some(account) => {
//             logs.add("account.bedrock", account.is_bedrock());
//             Some(acquire_member(&mut conn, &account, &body).await?)
//         }
//     };

//     let payload = Json(SessionGranted {
//         last_login_at: None,
//         perks: Vec::new(),
//         member,
//     });

//     Ok((StatusCode::CREATED, payload).into_response())
// }

// async fn acquire_member(
//     conn: &mut eden_sqlite::Connection,
//     account: &McAccount,
//     body: &RequestSession,
// ) -> ApiResult<MemberView> {
//     if account.is_bedrock() != body.bedrock {
//         return Err(ApiError::from_static(
//             ErrorCode::InvalidRequest,
//             "Incompatible Minecraft account type",
//         )
//         .into());
//     }

//     let member = Member::find_by_discord_user_id(conn, account.discord_user_id).await?;
//     Ok(member.into())
// }
