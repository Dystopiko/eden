use axum::{
    extract::{Extension, FromRequestParts, MatchedPath, Request},
    http::{HeaderName, HeaderValue, Method, StatusCode, Uri},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dashmap::DashMap;
use std::{sync::Arc, time::Instant};
use tracing::Instrument;
use uuid::Uuid;

use crate::middleware::extract_client_ip::ClientIp;

/// A request-scoped unique identifier injected by [`middleware`] into both
/// request and response extensions.
///
/// Included in error responses as `X-Request-ID` so that clients can correlate
/// a reported problem with server-side logs.
#[derive(Clone, Copy)]
pub struct RequestId(pub Uuid);

#[derive(FromRequestParts)]
pub struct RequestMetadata {
    client_ip: Extension<ClientIp>,
    method: Method,
    uri: Uri,
    matched_path: Option<Extension<MatchedPath>>,
    user_agent: Option<TypedHeader<UserAgent>>,
}

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Records a structured tracing span for each incoming HTTP request and appends
/// an `X-Request-ID` header to non-routing-failure responses.
pub async fn middleware(
    metadata: RequestMetadata,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    req.extensions_mut().insert(RequestId(id));

    let request_logs = RequestLogs::default();
    req.extensions_mut().insert(request_logs.clone());

    let matched_path = metadata
        .matched_path
        .as_ref()
        .map(|p| p.0.as_str())
        .unwrap_or_default();

    let user_agent = metadata.user_agent.as_ref().map(|v| v.as_str());

    let span = tracing::info_span!(
        "http.request",
        client.ip = %**metadata.client_ip,
        request.id = %id,
        request.method = %metadata.method,
        request.uri = %metadata.uri,
        request.path = %matched_path,
        request.user_agent = ?user_agent,
        request.metadata = tracing::field::Empty,
    );

    let start = Instant::now();
    let mut response = next.run(req).instrument(span.clone()).await;
    let duration = start.elapsed();

    // Omit request IDs from generic routing failures — these are not
    // correlated with any server-side work worth tracing.
    let status = response.status();
    if status != StatusCode::NOT_FOUND && status != StatusCode::METHOD_NOT_ALLOWED {
        let header_value = HeaderValue::from_str(&id.to_string())
            .expect("UUID should always produce a valid UTF-8 string");

        response.extensions_mut().insert(RequestId(id));
        response.headers_mut().insert(X_REQUEST_ID, header_value);
    }

    if !span.is_disabled() {
        let logged_metadata =
            serde_json::to_string(&*request_logs).unwrap_or_else(|_| String::from("{}"));

        span.record("request.metadata", tracing::field::display(logged_metadata));
    }

    span.in_scope(|| {
        tracing::trace!(
            "{method} {uri} -> {status} ({duration:?})",
            method = metadata.method,
            uri = metadata.uri,
            status = status.as_str(),
        );
    });

    response
}

/// A request-scoped key-value store for attaching structured metadata to the
/// active tracing span.
///
/// Injected into request extensions by [`middleware`] and available to
/// controllers via [`Extension<RequestLogs>`].
#[derive(Clone, Debug, Default)]
pub struct RequestLogs(Arc<DashMap<&'static str, String>>);

impl RequestLogs {
    /// Inserts or updates a metadata entry with the given key and value.
    pub fn add<V: std::fmt::Display>(&self, key: &'static str, value: V) {
        self.0.insert(key, value.to_string());
    }
}

impl std::ops::Deref for RequestLogs {
    type Target = DashMap<&'static str, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
