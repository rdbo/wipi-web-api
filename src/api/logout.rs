use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    api::Result,
    extractor::{RouterClient, UserSession},
    service::AuthService,
};

#[derive(Serialize)]
pub struct PostResponseBody {
    result: String,
}

pub async fn post(
    router_client: RouterClient,
    user_session: UserSession, // Force an authenticated user
    Extension(auth_service): Extension<Arc<AuthService>>,
) -> Result<impl IntoResponse> {
    log::info!(
        "Router client '{}' (MAC: {}) signed out with session '{}'",
        router_client.ip_address,
        router_client.mac_address,
        user_session.session_id
    );
    auth_service.sign_out()?;

    Ok(Json(PostResponseBody {
        result: "OK".to_owned(),
    }))
}
