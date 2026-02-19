mod interface;
mod route;
mod wiphy;

pub use interface::*;
pub use route::LinkState;

use crate::service::netlink::{
    route::{RouteInterface, RouteInterfaceKind, RouteManager},
    wiphy::{WiphyInterface, WiphyManager},
};
use anyhow::{Result, anyhow};
use futures_util::TryStreamExt;
use macaddr::MacAddr;
use rtnetlink::packet_route::{
    link::{LinkAttribute, LinkFlags, LinkLayerType},
    neighbour::{NeighbourAddress, NeighbourAttribute},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::IpAddr, ops::Index, str::FromStr};
use tokio::task::JoinHandle;
use wl_nl80211::{Nl80211Attr, Nl80211IfMode, Nl80211InterfaceType};

pub struct NetlinkService {
    wiphy_mgr: WiphyManager,
    route_mgr: RouteManager,
}

impl NetlinkService {
    pub fn try_new() -> Result<Self> {
        let wiphy_mgr = WiphyManager::try_new()?;
        let route_mgr = RouteManager::try_new()?;

        Ok(Self {
            wiphy_mgr,
            route_mgr,
        })
    }

    pub async fn get_interfaces(&self) -> Result<Vec<NetlinkInterface>> {
        // Handle wireless interfaces
        let wiphy_device_modes = self
            .wiphy_mgr
            .get_wiphy_devices()
            .await?
            .into_iter()
            .map(|x| {
                (
                    x.phy_index,
                    x.supported_iftypes
                        .into_iter()
                        .map(NetlinkInterfaceMode::from)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();
        let wiphy_interfaces = self.wiphy_mgr.get_wiphy_interfaces().await?;
        let mut interfaces = HashMap::<String, NetlinkInterface>::new();

        for iface in wiphy_interfaces {
            let supported_modes = wiphy_device_modes
                .get(&iface.phy_index)
                .cloned()
                .unwrap_or_else(|| {
                    log::error!(
                        "Failed to get wireless physical device '{}' for interface: '{}'",
                        iface.phy_index,
                        iface.name
                    );
                    vec![]
                });
            let active_mode: NetlinkInterfaceMode = iface.iftype.into();

            interfaces.insert(
                iface.name.clone(),
                NetlinkInterface {
                    index: iface.index,
                    name: iface.name,
                    kind: RouteInterfaceKind::Wireless,
                    link_flags: LinkFlags::empty(),
                    mode_status: Some(NetlinkInterfaceModeStatus {
                        active: active_mode,
                        supported: supported_modes,
                    }),
                },
            );
        }

        // Handle other interfaces
        let route_interfaces = self.route_mgr.get_interfaces().await?;
        for iface in route_interfaces {
            if let Some(inserted_iface) = interfaces.get_mut(&iface.name) {
                inserted_iface.link_flags = iface.link_flags;
                log::debug!(
                    "Interface '{}' already inserted in the interface map. Its data has been complemented with route information.",
                    iface.name
                );
                continue;
            }

            interfaces.insert(
                iface.name.clone(),
                NetlinkInterface {
                    index: iface.index,
                    name: iface.name,
                    kind: iface.kind,
                    link_flags: iface.link_flags,
                    mode_status: None,
                },
            );
        }

        Ok(interfaces.into_values().collect())
    }

    pub async fn get_neighbor_mac_addresses(&self) -> Result<HashMap<IpAddr, MacAddr>> {
        self.route_mgr.get_neighbor_mac_addresses().await
    }

    pub async fn find_interface_by_name(&self, name: &str) -> Result<NetlinkInterface> {
        // TODO: Avoid querying all interfaces - can be optimized with filters
        self.get_interfaces()
            .await?
            .into_iter()
            .find(|x| x.name == name)
            .ok_or(anyhow!("Could not find interface with name: {}", name))
    }

    pub async fn set_interface_state(
        &self,
        interface: &NetlinkInterface,
        state: LinkState,
    ) -> Result<()> {
        let route_interface = interface.to_owned().into();
        self.route_mgr.set_link_state(&route_interface, state).await
    }

    pub async fn set_interface_mode(
        &self,
        interface: &NetlinkInterface,
        mode: NetlinkInterfaceMode,
    ) -> Result<()> {
        let wiphy_interface = self
            .wiphy_mgr
            .get_wiphy_interfaces()
            .await?
            .into_iter()
            .find(|x| x.index == interface.index)
            .ok_or(anyhow!("Cannot set mode for interface: {:?}", interface))?;

        self.wiphy_mgr
            .set_wiphy_interface_mode(&wiphy_interface, mode.try_into()?)
            .await
    }
}
