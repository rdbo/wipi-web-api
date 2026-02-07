use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    api::Result,
    error::Error,
    extractor::{RouterClient, UserSession},
    service::{AuthService, NetlinkInterface, NetlinkService},
};

#[derive(Serialize)]
pub struct PostResponseBody {
    interfaces: Vec<NetlinkInterface>,
}

pub async fn post(
    _user_session: UserSession, // Force an authenticated user
    Extension(netlink_service): Extension<Arc<NetlinkService>>,
) -> Result<impl IntoResponse> {
    let interfaces = netlink_service
        .get_interfaces()
        .await
        .map_err(|_| Error::UnexpectedError)?;

    Ok(Json(PostResponseBody { interfaces }))
}
