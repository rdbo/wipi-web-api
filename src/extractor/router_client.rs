use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use macaddr::{MacAddr, MacAddr6};

use crate::error::Error;

pub struct RouterClient {
    pub ip_address: IpAddr,
    pub mac_address: MacAddr,
}

impl<S> FromRequestParts<S> for RouterClient
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ConnectInfo(socket_addr) = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or(Error::RouterClientIdentificationFailed)?;

        let mut ip_address = socket_addr.ip();

        // Resolve reverse proxy
        if ip_address.is_loopback()
            && let Some(real_ip) = parts.headers.get("X-Real-IP")
        {
            ip_address = real_ip
                .to_str()
                .ok()
                .and_then(|s| s.parse::<IpAddr>().ok())
                .ok_or(Error::RouterClientIdentificationFailed)?;
        }
        // TODO: Actually get MAC address
        let mac_address = MacAddr::V6(MacAddr6::nil());

        Ok(RouterClient {
            ip_address,
            mac_address,
        })
    }
}
