use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
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

use crate::{error::Error, service::NetlinkService};

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

        let netlink_service = parts
            .extensions
            .get::<Arc<NetlinkService>>()
            .ok_or(Error::RouterClientIdentificationFailed)?;

        let ip_address = socket_addr.ip();

        // Get MAC address
        log::trace!(
            "Searching for the MAC address of the IP '{}'...",
            ip_address
        );
        let mut mac_table = netlink_service
            .get_neighbor_mac_addresses()
            .await
            .map_err(|_| Error::RouterClientIdentificationFailed)?;

        let mac_address = mac_table
            .remove(&ip_address) // Get owned value
            .ok_or(Error::RouterClientIdentificationFailed)?;
        log::debug!(
            "IP address '{}' has the MAC address '{}'",
            ip_address,
            mac_address
        );

        Ok(RouterClient {
            ip_address,
            mac_address,
        })
    }
}
