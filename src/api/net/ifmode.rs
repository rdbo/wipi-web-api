use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    api::Result,
    error::Error,
    extractor::{RouterClient, UserSession},
    service::{
        AuthService, LinkFlagsStruct, LinkState, NetlinkInterface, NetlinkInterfaceMode,
        NetlinkService,
    },
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRequestBody {
    interface_name: String,
    interface_mode: NetlinkInterfaceMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponseBody {
    interface_mode: NetlinkInterfaceMode,
}

pub async fn post(
    _user_session: UserSession, // Force an authenticated user
    Extension(netlink_service): Extension<Arc<NetlinkService>>,
    Json(payload): Json<PostRequestBody>,
) -> Result<impl IntoResponse> {
    let interface = netlink_service
        .find_interface_by_name(&payload.interface_name)
        .await
        .map_err(|_| Error::InterfaceNotFound)?;

    netlink_service
        .set_interface_mode(&interface, payload.interface_mode)
        .await
        .map_err(|e| {
            log::error!("Failed to set interface mode: {}", e);
            Error::UnexpectedError
        })?;

    let interface = netlink_service
        .find_interface_by_name(&payload.interface_name)
        .await
        .map_err(|e| {
            log::error!("Failed to find interface after setting mode: {}", e);
            Error::UnexpectedError
        })?;

    Ok(Json(PostResponseBody {
        interface_mode: interface
            .mode_status
            .map(|x| x.active)
            .ok_or(Error::UnexpectedError)?,
    }))
}
