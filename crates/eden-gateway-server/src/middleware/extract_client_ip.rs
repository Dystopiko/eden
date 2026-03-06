use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};

#[derive(Clone, Copy, Debug)]
pub struct ClientIp(IpAddr);

impl std::ops::Deref for ClientIp {
    type Target = IpAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn middleware(mut req: Request, next: Next) -> Response {
    // TODO: Add support for `X-Forwarded-For` and `X-Real-Ip` headers
    //       as another methods of extracting client's true IP address
    let ConnectInfo(peer_addr) = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .unwrap_or_else(|| {
            panic!(
                "Please use `into_make_service_with_connect_info` as this middleware \
                depends on it to acquire client's IP addresses."
            )
        });

    let peer_addr = peer_addr.ip();
    req.extensions_mut().insert(ClientIp(peer_addr));

    next.run(req).await
}
