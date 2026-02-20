use std::collections::HashMap;

use anyhow::Result;
use futures_util::TryStreamExt;
use tokio::task::JoinHandle;
use wl_nl80211::{
    Nl80211Attr, Nl80211IfMode, Nl80211Interface, Nl80211InterfaceType, Nl80211NewInterface,
};
use wl_nl80211::{Nl80211Handle, Nl80211Message};

#[derive(Debug, Clone)]
pub struct WiphyInterface {
    pub index: u32,
    pub phy_index: u32,
    pub name: String,
    pub iftype: Nl80211InterfaceType,
}

#[derive(Debug, Clone)]
pub struct WiphyDevice {
    pub phy_index: u32,
    pub phy_name: String,
    pub supported_iftypes: Vec<Nl80211IfMode>,
}

pub struct WiphyManager {
    nl80211: Nl80211Handle,
    nl80211_future: JoinHandle<()>,
}

impl WiphyManager {
    pub fn try_new() -> Result<Self> {
        let (connection, nl80211, _) = wl_nl80211::new_connection()?;
        let nl80211_future = tokio::spawn(connection);
        Ok(Self {
            nl80211,
            nl80211_future,
        })
    }
    pub async fn get_wiphy_interfaces(&self) -> Result<Vec<WiphyInterface>> {
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

    pub async fn get_wiphy_devices(&self) -> Result<Vec<WiphyDevice>> {
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

    pub async fn set_wiphy_interface_mode(
        &self,
        wiphy_interface: &WiphyInterface,
        iftype: Nl80211InterfaceType,
    ) -> Result<()> {
        let attrs = wl_nl80211::Nl80211AttrsBuilder::new()
            .if_index(wiphy_interface.index)
            .interface_type(iftype)
            .build();
        let mut result = self.nl80211.interface().set(attrs).execute().await;
        result.try_next().await?;
        Ok(())
    }

    pub async fn create_wiphy_interface(
        &self,
        wiphy_dev: &WiphyDevice,
        iftype: Nl80211InterfaceType,
        name: String,
    ) -> Result<()> {
        self.nl80211
            .interface()
            .add(Nl80211NewInterface::new(wiphy_dev.phy_index, iftype, name).build())
            .execute()
            .await
            .try_next()
            .await?;

        Ok(())
    }

    pub async fn delete_wiphy_interface(&self, wiphy_iface: &WiphyInterface) -> Result<()> {
        self.nl80211
            .interface()
            .delete(Nl80211Interface::new(wiphy_iface.index).build())
            .execute()
            .await
            .try_next()
            .await?;

        Ok(())
    }
}

impl Drop for WiphyManager {
    fn drop(&mut self) {
        self.nl80211_future.abort();
    }
}
