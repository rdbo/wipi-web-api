use std::{collections::HashMap, net::IpAddr, str::FromStr};

use anyhow::{Result, anyhow};
use futures_util::TryStreamExt;
use macaddr::MacAddr;
use rtnetlink::packet_route::{
    link::LinkAttribute,
    neighbour::{NeighbourAddress, NeighbourAttribute},
};
use serde::Serialize;
use tokio::task::JoinHandle;

pub struct NetlinkService {
    rtnetlink_future: JoinHandle<()>,
    rtnetlink_handle: rtnetlink::Handle,
}

#[derive(Serialize)]
pub struct NetlinkInterface {
    name: String,
}

impl NetlinkService {
    pub fn try_new() -> std::io::Result<Self> {
        let (connection, rtnetlink_handle, _) = rtnetlink::new_connection()?;
        let rtnetlink_future = tokio::spawn(connection);
        Ok(Self {
            rtnetlink_future,
            rtnetlink_handle,
        })
    }

    pub async fn get_interfaces(&self) -> Result<Vec<NetlinkInterface>> {
        let mut links = self.rtnetlink_handle.link().get().execute();
        let mut interfaces = Vec::new();
        while let Some(link) = links.try_next().await? {
            let Some(ifname) = link.attributes.into_iter().find_map(|x| {
                if let LinkAttribute::IfName(name) = x {
                    Some(name)
                } else {
                    None
                }
            }) else {
                // TODO: Assure that skipping unnamed interfaces is a good idea
                log::trace!("Unnamed interface found! Index: {}", link.header.index);
                continue;
            };

            log::trace!("Found interface: {}", ifname);

            interfaces.push(NetlinkInterface { name: ifname });
        }

        Ok(interfaces)
    }

    pub async fn get_neighbor_mac_addresses(&self) -> Result<HashMap<IpAddr, MacAddr>> {
        let mut address_map = HashMap::new();

        let mut neighbours = self.rtnetlink_handle.neighbours().get().execute();
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

impl Drop for NetlinkService {
    fn drop(&mut self) {
        self.rtnetlink_future.abort();
    }
}
