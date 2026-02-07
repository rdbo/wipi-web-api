use std::{collections::HashMap, net::IpAddr, str::FromStr};

use anyhow::Result;
use axum::routing::RouterIntoService;
use futures_util::TryStreamExt;
use macaddr::MacAddr;
use rtnetlink::packet_route::{
    link::{LinkAttribute, LinkLayerType},
    neighbour::{NeighbourAddress, NeighbourAttribute},
};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub enum RouteInterfaceKind {
    Ethernet,
    Wireless,
    Loopback,
    Unknown(u16),
}

#[derive(Debug, Clone)]
pub struct RouteInterface {
    pub index: u32,
    pub name: String,
    pub kind: RouteInterfaceKind,
}

pub struct RouteManager {
    rtnetlink_future: JoinHandle<()>,
    rtnetlink: rtnetlink::Handle,
}

impl RouteManager {
    pub fn try_new() -> Result<Self> {
        let (connection, rtnetlink, _) = rtnetlink::new_connection()?;
        let rtnetlink_future = tokio::spawn(connection);
        Ok(Self {
            rtnetlink_future,
            rtnetlink,
        })
    }

    pub async fn get_interfaces(&self) -> Result<Vec<RouteInterface>> {
        let mut links = self.rtnetlink.link().get().execute();
        let mut interfaces = Vec::new();

        while let Some(link) = links.try_next().await? {
            let index = link.header.index;
            let Some(ifname) = link.attributes.into_iter().find_map(|x| {
                if let LinkAttribute::IfName(name) = x {
                    Some(name)
                } else {
                    None
                }
            }) else {
                // TODO: Assure that skipping unnamed interfaces is a good idea
                log::warn!("Unnamed interface found! Index: {}", index);
                continue;
            };

            log::trace!("Found interface: {}", ifname);

            let kind = match link.header.link_layer_type {
                LinkLayerType::Ether => RouteInterfaceKind::Ethernet,
                LinkLayerType::Loopback => RouteInterfaceKind::Loopback,
                LinkLayerType::Ieee80211
                | LinkLayerType::Ieee80211Radiotap
                | LinkLayerType::Ieee80211Prism => RouteInterfaceKind::Wireless,
                other => {
                    log::warn!("Unknown interface kind: {other:?}");
                    RouteInterfaceKind::Unknown(other as u16)
                }
            };

            interfaces.push(RouteInterface {
                index,
                name: ifname,
                kind,
            });
        }

        Ok(interfaces)
    }

    pub async fn get_neighbor_mac_addresses(&self) -> Result<HashMap<IpAddr, MacAddr>> {
        let mut address_map = HashMap::new();

        let mut neighbours = self.rtnetlink.neighbours().get().execute();
        while let Some(route) = neighbours.try_next().await? {
            log::trace!("Current route: {:?}", route);

            let mut ip_address = None;
            let mut mac_address = None;
            for attr in route.attributes.into_iter() {
                match attr {
                    NeighbourAttribute::Destination(NeighbourAddress::Inet(ip)) => {
                        log::trace!("Route IPv4 address: {:?}", ip);
                        ip_address = Some(IpAddr::V4(ip.to_owned()));
                    }
                    NeighbourAttribute::Destination(NeighbourAddress::Inet6(ip)) => {
                        log::trace!("Route IPv6 address: {:?}", ip);
                        ip_address = Some(IpAddr::V6(ip.to_owned()));
                    }
                    NeighbourAttribute::LinkLocalAddress(addr) => {
                        log::trace!("LinkLocalAddress: {:?}", addr);
                        let mac_str = addr
                            .into_iter()
                            .map(|byte| format!("{:02X}", byte))
                            .collect::<Vec<_>>()
                            .join(":");
                        mac_address = MacAddr::from_str(mac_str.as_str()).ok();
                    }
                    _ => {
                        continue;
                    }
                }
            }

            let Some(ip_address) = ip_address else {
                log::trace!("No IP address in route, skipping...");
                continue;
            };

            let Some(mac_address) = mac_address else {
                log::trace!("No MAC address in route, skipping...");
                continue;
            };

            address_map.insert(ip_address, mac_address);
        }

        Ok(address_map)
    }
}

impl Drop for RouteManager {
    fn drop(&mut self) {
        self.rtnetlink_future.abort();
    }
}
