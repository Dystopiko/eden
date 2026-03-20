use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eden_core::jobs::OnPlayerJoined;
use eden_database::{
    Timestamp,
    primary_guild::{LoggedInEvent, McAccountType, logged_in_event::NewLoggedInEvent},
    views::mc_account::McAccountView,
};
use eden_gateway_api::{
    member::EncodedMember,
    sessions::request::{RequestSession, SessionGranted},
};
use eden_sqlite::error::QueryResultExt;
use std::{net::IpAddr, sync::Arc};
use uuid::Uuid;

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
    let mut conn = kernel.pools.db_read_prefer_primary().await?;
    let session = grant_session(&mut conn, &body, &logs).await?;

    // Make sure player is not logging in too frequently by determining
    // their IP address or the player's discord ID.
    let rl_actor = session.actor();
    let rl_action = session.limited_action();

    logs.add("ratelimit.action", format!("{rl_action:?}"));
    logs.add("ratelimit.actor", format!("{rl_actor:?}"));

    let rl_headers = rate_limiter.permit(rl_actor, rl_action)?.into_headers();
    let event = session.new_logged_in_event();
    let response = session.response();

    kernel.enqueue_job(OnPlayerJoined(event)).await?;
    Ok((StatusCode::CREATED, rl_headers, Json(response)).into_response())
}

async fn grant_session(
    conn: &mut eden_sqlite::Connection,
    body: &RequestSession,
    logs: &RequestLogs,
) -> ApiResult<Session> {
    let account_type = if body.java {
        McAccountType::Java
    } else {
        McAccountType::Bedrock
    };

    let account = McAccountView::find_by_mc_uuid(conn, body.uuid)
        .await
        .optional()?;

    logs.add("account.exists", account.is_some());
    let Some(account) = account else {
        return Ok(Session::guest(body.ip, account_type, body.uuid));
    };

    logs.add("account.java", account.kind.is_java());
    if account.kind != account_type {
        return Err(ApiError::from_static(
            ErrorCode::InvalidRequest,
            "Incompatible account type",
        ));
    }

    Ok(Session::member(body.ip, account))
}

struct Session {
    ip_addr: IpAddr,
    view: GuestOrMember,
}

enum GuestOrMember {
    Guest {
        account_type: McAccountType,
        uuid: Uuid,
    },
    Member(McAccountView),
}

impl GuestOrMember {
    #[must_use]
    const fn last_login_at(&self) -> Option<Timestamp> {
        match self {
            Self::Guest { .. } => None,
            Self::Member(view) => view.last_login_at,
        }
    }

    #[must_use]
    const fn as_member(&self) -> Option<&McAccountView> {
        match self {
            Self::Guest { .. } => None,
            Self::Member(view) => Some(view),
        }
    }

    #[must_use]
    const fn uuid(&self) -> Uuid {
        match self {
            Self::Guest { uuid, .. } => *uuid,
            Self::Member(view) => view.uuid,
        }
    }

    #[must_use]
    const fn mc_account_type(&self) -> McAccountType {
        match self {
            Self::Guest { account_type, .. } => *account_type,
            Self::Member(view) => view.kind,
        }
    }

    #[must_use]
    fn encode(self) -> Option<EncodedMember> {
        match self {
            Self::Guest { .. } => None,
            Self::Member(view) => Some(view.into()),
        }
    }
}

impl Session {
    #[must_use]
    const fn guest(ip_addr: IpAddr, account_type: McAccountType, uuid: Uuid) -> Self {
        Self {
            ip_addr,
            view: GuestOrMember::Guest { account_type, uuid },
        }
    }

    #[must_use]
    const fn member(ip_addr: IpAddr, view: McAccountView) -> Self {
        Self {
            ip_addr,
            view: GuestOrMember::Member(view),
        }
    }

    #[must_use]
    const fn is_guest(&self) -> bool {
        matches!(self.view, GuestOrMember::Guest { .. })
    }

    #[must_use]
    const fn limited_action(&self) -> LimitedAction {
        LimitedAction::RequestSession {
            guest: self.is_guest(),
        }
    }

    #[must_use]
    const fn actor(&self) -> Actor {
        match &self.view {
            GuestOrMember::Member(view) => Actor::Member(view.member_id.into_inner().cast()),
            GuestOrMember::Guest { .. } => Actor::Ip(self.ip_addr),
        }
    }

    fn new_logged_in_event(&self) -> NewLoggedInEvent {
        let member = self.view.as_member();
        let member_id = member.map(|v| v.member_id.cast());
        let username = member.map(|v| v.username.to_owned());

        LoggedInEvent::new_event()
            .ip_address(self.ip_addr)
            .kind(self.view.mc_account_type())
            .player_uuid(self.view.uuid())
            .maybe_member_id(member_id)
            .maybe_username(username)
            .build()
    }

    fn response(self) -> SessionGranted {
        SessionGranted {
            last_login_at: self.view.last_login_at(),
            member: self.view.encode(),
            perks: Vec::new(),
        }
    }
}
