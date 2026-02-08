use std::{collections::HashMap, net::IpAddr, str::FromStr};

use anyhow::{Result, anyhow};
use axum::routing::RouterIntoService;
use futures_util::TryStreamExt;
use macaddr::MacAddr;
use rtnetlink::{
    LinkUnspec,
    packet_route::{
        link::{LinkAttribute, LinkHeader, LinkLayerType, LinkMessage, State},
        neighbour::{NeighbourAddress, NeighbourAttribute},
    },
};
use serde::Serialize;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize)]
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
    pub oper_state: OperState,
}

#[derive(Debug, Clone, Serialize)]
pub enum OperState {
    Unknown,
    Down,
    Up,
    Other(u8),
}

impl From<State> for OperState {
    fn from(value: State) -> Self {
        match value {
            State::Down => Self::Down,
            State::Up => Self::Up,
            x => Self::Other(x.into()),
        }
    }
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
            let mut ifname = None;
            let mut oper_state = None;

            for attr in link.attributes {
                match attr {
                    LinkAttribute::IfName(name) => ifname = Some(name),
                    LinkAttribute::OperState(state) => oper_state = Some(state.into()),
                    _ => {}
                }
            }

            let Some(ifname) = ifname else {
                log::warn!("Unnamed interface found! Index: {}", index);
                continue;
            };

            let Some(oper_state) = oper_state else {
                log::warn!("Missing oper state for interface '{}'", ifname);
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
                oper_state,
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

    pub async fn set_link_oper_state(
        &self,
        route_interface: &RouteInterface,
        state: OperState,
    ) -> Result<()> {
        self.rtnetlink
            .link()
            .set(match state {
                OperState::Down => LinkUnspec::new_with_index(route_interface.index)
                    .down()
                    .build(),
                OperState::Up => LinkUnspec::new_with_index(route_interface.index)
                    .up()
                    .build(),
                _ => {
                    return Err(anyhow!(
                        "Invalid interface operational state specified: {:?}",
                        state
                    ));
                }
            })
            .execute()
            .await?;

        Ok(())
    }
}

impl Drop for RouteManager {
    fn drop(&mut self) {
        self.rtnetlink_future.abort();
    }
}
