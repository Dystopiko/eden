use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_database::primary_guild::{McAccount, McAccountChallenge};
use eden_gateway_api::members::link::minecraft::{LinkChallenge, LinkMcAccount};
use eden_sqlite::error::QueryResultExt;
use eden_text_handling::generator::random_words;
use erased_report::ErasedReport;
use error_stack::ResultExt;
use sha2::Digest;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::task::spawn_blocking;

use crate::{
    ApiError,
    controllers::ApiResult,
    errors::ErrorCode,
    extract::{Kernel, Validated},
    middleware::trace_request::RequestLogs,
    ratelimiter::{Actor, LimitedAction, RateLimiter},
};

// This is to give time for both Java and Bedrock players to solve the challenge
static DEFAULT_CHALLENGE_TTL: Duration = Duration::from_mins(10);

pub async fn minecraft(
    Kernel(kernel): Kernel,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    Extension(logs): Extension<RequestLogs>,
    Validated(body): Validated<Json<LinkMcAccount>>,
) -> ApiResult<Response> {
    let rl_actor = Actor::Ip(body.ip);
    logs.add("ratelimit.actor", format!("{rl_actor:?}"));

    let rl_headers = rate_limiter
        .permit(rl_actor, LimitedAction::LinkMinecraftAccount)?
        .into_headers();

    let mut conn = kernel.pools.db_write().await?;

    // Do we have existing member data from UUID?
    let already_linked = McAccount::find_by_uuid(&mut conn, body.uuid)
        .await
        .optional()?
        .is_some();

    if already_linked {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "You already linked your Minecraft account to your Discord account!",
        ));
    }

    // Make sure we don't have duplicated challenges within the ttl period
    let has_existing = McAccountChallenge::find_in_progress(&mut conn, body.uuid)
        .await
        .optional()?
        .is_some();

    if has_existing {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "You have already requested to link your Minecraft account. Please send the \
            code to Eden Discord bot in direct message or wait to be expired.",
        ));
    }

    let code = generate_challenge_code().await?;

    // TODO: Make every hashed code to avoid guessing attacks
    let mut hasher = sha2::Sha256::new();
    hasher.update(&code);

    let hashed_code = hex::encode(hasher.finalize());
    logs.add("challenge.hashed_code", hashed_code.to_owned());

    let (challenge_id, expires_at) = McAccountChallenge::new_challenge()
        .ip_address(body.ip)
        .username(&body.username)
        .uuid(body.uuid)
        .hashed_code(&hashed_code)
        .ttl(DEFAULT_CHALLENGE_TTL)
        .java(body.java)
        .build()
        .insert(&mut conn)
        .await?;

    logs.add("challenge.id", format!("{challenge_id}"));
    let response: LinkChallenge = LinkChallenge { code, expires_at };

    conn.commit().await.map_err(ErasedReport::new)?;
    Ok((StatusCode::CREATED, rl_headers, Json(response)).into_response())
}

async fn generate_challenge_code() -> ApiResult<String> {
    // Minimum characters to have a challenge code.
    const MIN_CHARS: usize = 25;

    #[derive(Debug, Error)]
    #[error("Challenge code generator got panicked")]
    struct GeneratorPanicked;

    spawn_blocking(|| {
        let mut rng = rand::rng();
        let mut code = String::with_capacity(MIN_CHARS);
        for word in random_words(&mut rng) {
            if code.chars().count() != 0 {
                code.push('-');
            }
            code.push_str(word);

            if code.len() >= MIN_CHARS {
                break;
            }
        }
        code
    })
    .await
    .change_context(GeneratorPanicked)
    .map_err(Into::into)
}
