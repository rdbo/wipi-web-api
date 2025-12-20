use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use futures_util::stream::TryStreamExt;
use macaddr::{MacAddr, MacAddr6};
use rtnetlink::packet_route::link::LinkAttribute;
use rtnetlink::packet_route::neighbour::NeighbourAddress;
use rtnetlink::{Handle, packet_route::neighbour::NeighbourAttribute};

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

        let nl_handle = parts
            .extensions
            .get::<Handle>()
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

        // Get MAC address
        log::debug!(
            "Searching for the MAC address of the IP '{}'...",
            ip_address
        );

        let mut mac_address: Result<MacAddr, Error> = Err(Error::RouterClientIdentificationFailed);

        let mut neighbours = nl_handle.neighbours().get().execute();
        while let Some(route) = neighbours
            .try_next()
            .await
            .map_err(|_| Error::RouterClientIdentificationFailed)?
        {
            log::trace!("Current route: {:?}", route);
            let Some(route_ip_address) = route.attributes.iter().find_map(|attr| match attr {
                NeighbourAttribute::Destination(NeighbourAddress::Inet(ip)) => {
                    Some(IpAddr::V4(ip.to_owned()))
                }
                NeighbourAttribute::Destination(NeighbourAddress::Inet6(ip)) => {
                    Some(IpAddr::V6(ip.to_owned()))
                }
                _ => None,
            }) else {
                log::trace!("Missing IP in route");
                continue;
            };

            if route_ip_address != ip_address {
                log::trace!(
                    "Route IP address '{}' doesnt match wanted IP address '{}'",
                    route_ip_address,
                    ip_address
                );
                continue;
            }

            log::trace!(
                "Route IP address matches request IP address: {}",
                ip_address
            );

            mac_address = route
                .attributes
                .iter()
                .find_map(|attr| match attr {
                    NeighbourAttribute::LinkLocalAddress(addr) => {
                        log::trace!("LinkLocalAddress: {:?}", addr);
                        let mac_str = addr
                            .into_iter()
                            .map(|byte| format!("{:02X}", byte))
                            .collect::<Vec<_>>()
                            .join(":");
                        MacAddr::from_str(mac_str.as_str()).ok()
                    }
                    _ => None,
                })
                .ok_or(Error::RouterClientIdentificationFailed);

            break;
        }

        let mac_address = mac_address?;
        Ok(RouterClient {
            ip_address,
            mac_address,
        })
    }
}
