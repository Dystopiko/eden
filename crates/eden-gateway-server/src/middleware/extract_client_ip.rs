use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};

/// The IP address of the connected client, extracted from the TCP peer address.
///
/// Injected into request extensions by [`middleware`] and available downstream
/// via `Extension<ClientIp>`.
///
/// # Limitations
///
/// Currently only reads the raw TCP peer address.
#[derive(Clone, Copy, Debug)]
pub struct ClientIp(pub IpAddr);

impl std::ops::Deref for ClientIp {
    type Target = IpAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts the client IP from the TCP peer address and inserts [`ClientIp`]
/// into request extensions.
///
/// # Panics
///
/// Panics at startup if the server was not started with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
pub async fn middleware(mut req: Request, next: Next) -> Response {
    let ConnectInfo(peer_addr) = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .unwrap_or_else(|| {
            panic!(
                "`extract_client_ip` requires the server to be started with \
                `into_make_service_with_connect_info::<SocketAddr>()`"
            )
        });

    let extension = ClientIp(peer_addr.ip());
    req.extensions_mut().insert(extension);
    next.run(req).await
}
