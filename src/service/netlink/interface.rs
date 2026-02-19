use crate::service::{
    LinkState,
    netlink::{
        route::{RouteInterface, RouteInterfaceKind, RouteManager},
        wiphy::{WiphyInterface, WiphyManager},
    },
};
use anyhow::{Result, anyhow};
use futures_util::TryStreamExt;
use macaddr::MacAddr;
use rtnetlink::packet_route::{
    link::{LinkAttribute, LinkFlags, LinkLayerType},
    neighbour::{NeighbourAddress, NeighbourAttribute},
};
use serde::{Deserialize, Serialize, Serializer};
use std::{collections::HashMap, net::IpAddr, str::FromStr};
use tokio::task::JoinHandle;
use wl_nl80211::{Nl80211Attr, Nl80211IfMode, Nl80211InterfaceType};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum NetlinkInterfaceMode {
    Station,
    Monitor,
    AccessPoint,
    #[serde(skip_deserializing)]
    OtherWireless(u32),
}

impl From<Nl80211IfMode> for NetlinkInterfaceMode {
    fn from(value: Nl80211IfMode) -> Self {
        match value {
            Nl80211IfMode::Station => Self::Station,
            Nl80211IfMode::Monitor => Self::Monitor,
            Nl80211IfMode::Ap => Self::AccessPoint,
            other => Self::OtherWireless(u16::from(other).into()),
        }
    }
}

impl From<Nl80211InterfaceType> for NetlinkInterfaceMode {
    fn from(value: Nl80211InterfaceType) -> Self {
        match value {
            Nl80211InterfaceType::Station => Self::Station,
            Nl80211InterfaceType::Monitor => Self::Monitor,
            Nl80211InterfaceType::Ap => Self::AccessPoint,
            other => Self::OtherWireless(other.into()),
        }
    }
}

impl TryInto<Nl80211InterfaceType> for NetlinkInterfaceMode {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<Nl80211InterfaceType, Self::Error> {
        Ok(match self {
            Self::Station => Nl80211InterfaceType::Station,
            Self::Monitor => Nl80211InterfaceType::Monitor,
            Self::AccessPoint => Nl80211InterfaceType::Ap,
            Self::OtherWireless(other) => Nl80211InterfaceType::Other(other),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NetlinkInterfaceModeStatus {
    pub active: NetlinkInterfaceMode,
    pub supported: Vec<NetlinkInterfaceMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkFlagsStruct {
    is_up: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetlinkInterface {
    #[serde(skip)]
    pub index: u32,
    pub name: String,
    pub kind: RouteInterfaceKind,
    #[serde(serialize_with = "link_flags_serializer")]
    pub link_flags: LinkFlags,
    pub mode_status: Option<NetlinkInterfaceModeStatus>,
}

impl NetlinkInterface {
    pub fn state(&self) -> LinkState {
        if self.link_flags & LinkFlags::Up == LinkFlags::Up {
            LinkState::Up
        } else {
            LinkState::Down
        }
    }
}

impl Into<RouteInterface> for NetlinkInterface {
    fn into(self) -> RouteInterface {
        RouteInterface {
            index: self.index,
            name: self.name,
            kind: self.kind,
            link_flags: self.link_flags,
        }
    }
}

fn link_flags_serializer<S: Serializer>(link_flags: &LinkFlags, s: S) -> Result<S::Ok, S::Error> {
    LinkFlagsStruct {
        is_up: *link_flags & LinkFlags::Up == LinkFlags::Up,
    }
    .serialize(s)
}
