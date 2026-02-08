use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    api::Result,
    error::Error,
    extractor::{RouterClient, UserSession},
    service::{AuthService, NetlinkInterface, NetlinkService, OperState},
};

#[derive(Deserialize)]
pub enum IfState {
    Down,
    Up,
}

impl Into<OperState> for IfState {
    fn into(self) -> OperState {
        match self {
            IfState::Down => OperState::Down,
            IfState::Up => OperState::Up,
        }
    }
}

#[derive(Deserialize)]
pub struct PostRequestBody {
    interface_name: String,
    oper_state: IfState,
}

#[derive(Serialize)]
pub struct PostResponseBody {
    oper_state: OperState,
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
        .set_interface_oper_state(&interface, payload.oper_state.into())
        .await
        .map_err(|_| Error::UnexpectedError)?;

    let interface = netlink_service
        .find_interface_by_name(&payload.interface_name)
        .await
        .map_err(|_| Error::UnexpectedError)?;

    Ok(Json(PostResponseBody {
        oper_state: interface.oper_state,
    }))
}
