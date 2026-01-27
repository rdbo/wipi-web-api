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
use wl_nl80211::{Nl80211Attr, Nl80211IfMode, Nl80211InterfaceType};

pub struct NetlinkService {
    rtnetlink_future: JoinHandle<()>,
    rtnetlink: rtnetlink::Handle,
    nl80211_future: JoinHandle<()>,
    nl80211: wl_nl80211::Nl80211Handle,
}

#[derive(Debug)]
pub struct WiphyInterface {
    index: u32,
    phy_index: u32,
    name: String,
    iftype: Nl80211InterfaceType,
}

#[derive(Debug)]
pub struct WiphyDevice {
    phy_index: u32,
    phy_name: String,
    supported_iftypes: Vec<Nl80211IfMode>,
}

#[derive(Debug, Serialize)]
pub struct NetlinkInterface {
    index: u32,
    name: String,
}

impl NetlinkService {
    pub fn try_new() -> std::io::Result<Self> {
        let (connection, rtnetlink, _) = rtnetlink::new_connection()?;
        let rtnetlink_future = tokio::spawn(connection);
        let (connection, nl80211, _) = wl_nl80211::new_connection().unwrap();
        let nl80211_future = tokio::spawn(connection);

        Ok(Self {
            rtnetlink_future,
            rtnetlink,
            nl80211_future,
            nl80211,
        })
    }

    async fn get_wiphy_interfaces(&self) -> Result<Vec<WiphyInterface>> {
        let mut interfaces = vec![];
        let mut interface = self.nl80211.interface().get(Vec::new()).execute().await;
        while let Some(msg) = interface.try_next().await? {
            let mut index = None;
            let mut phy_index = None;
            let mut name = None;
            let mut iftype = None;
            for attr in msg.payload.attributes.into_iter() {
                match attr {
                    Nl80211Attr::IfIndex(i) => {
                        index = Some(i);
                    }
                    Nl80211Attr::Wiphy(i) => {
                        phy_index = Some(i);
                    }
                    Nl80211Attr::IfName(s) => {
                        name = Some(s);
                    }
                    Nl80211Attr::IfType(t) => {
                        iftype = Some(t);
                    }
                    _ => {}
                }
            }
            let (Some(index), Some(phy_index), Some(name), Some(iftype)) =
                (index, phy_index, name, iftype)
            else {
                log::warn!("Missing required field in wiphy interface.");
                continue;
            };
            interfaces.push(WiphyInterface {
                index,
                phy_index,
                name,
                iftype,
            })
        }

        Ok(interfaces)
    }

    async fn get_wiphy_devices(&self) -> Result<Vec<WiphyDevice>> {
        let mut wiphy = self.nl80211.wireless_physic().get().execute().await;
        let mut devices = HashMap::new();
        while let Some(msg) = wiphy.try_next().await? {
            let mut phy_index = None;
            let mut phy_name = None;
            let mut supported_iftypes = None;
            for attr in msg.payload.attributes.into_iter() {
                match attr {
                    wl_nl80211::Nl80211Attr::Wiphy(index) => {
                        phy_index = Some(index);
                    }
                    wl_nl80211::Nl80211Attr::WiphyName(name) => {
                        phy_name = Some(name);
                    }
                    wl_nl80211::Nl80211Attr::SupportedIftypes(iftypes) => {
                        supported_iftypes = Some(iftypes);
                    }

                    _ => {}
                }
            }

            let Some(phy_index) = phy_index else {
                log::warn!("Missing wireless physical device index");
                continue;
            };

            let mut wiphy_dev = devices.remove(&phy_index).unwrap_or_else(|| WiphyDevice {
                phy_index,
                phy_name: "".to_string(),
                supported_iftypes: vec![],
            });

            if let Some(phy_name) = phy_name {
                wiphy_dev.phy_name = phy_name;
            };

            if let Some(supported_iftypes) = supported_iftypes {
                wiphy_dev.supported_iftypes = supported_iftypes;
            }

            devices.insert(phy_index, wiphy_dev);
        }

        Ok(devices.into_values().collect())
    }

    pub async fn get_interfaces(&self) -> Result<Vec<NetlinkInterface>> {
        let mut links = self.rtnetlink.link().get().execute();
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
                log::warn!("Unnamed interface found! Index: {}", link.header.index);
                continue;
            };

            log::trace!("Found interface: {}", ifname);

            interfaces.push(NetlinkInterface {
                index: link.header.index,
                name: ifname,
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

impl Drop for NetlinkService {
    fn drop(&mut self) {
        self.rtnetlink_future.abort();
        self.nl80211_future.abort();
    }
}
