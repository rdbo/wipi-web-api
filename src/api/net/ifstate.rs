use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    api::Result,
    error::Error,
    extractor::{RouterClient, UserSession},
    service::{AuthService, LinkState, NetlinkInterface, NetlinkService},
};

#[derive(Deserialize)]
pub struct PostRequestBody {
    interface_name: String,
    link_state: LinkState,
}

#[derive(Serialize)]
pub struct PostResponseBody {
    link_state: LinkState,
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
        .set_interface_state(&interface, payload.link_state)
        .await
        .map_err(|_| Error::UnexpectedError)?;

    let interface = netlink_service
        .find_interface_by_name(&payload.interface_name)
        .await
        .map_err(|_| Error::UnexpectedError)?;

    Ok(Json(PostResponseBody {
        link_state: interface.state(),
    }))
}
