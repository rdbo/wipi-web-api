use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{error::Error, extractor::RouterClient, service::AuthService};

#[derive(Deserialize)]
pub struct PostRequestBody {
    password: String,
}

#[derive(Serialize)]
pub struct PostResponseBody {
    session_id: String,
}

pub async fn post(
    router_client: RouterClient,
    State(auth_service): State<Arc<AuthService>>,
    Json(PostRequestBody { password }): Json<PostRequestBody>,
) -> Result<impl IntoResponse, Error> {
    log::trace!(
        "Router client '{}' (MAC: {}) attemping login...",
        router_client.ip_address,
        router_client.mac_address
    );
    let session_id = auth_service.try_login(password)?.to_string();

    Ok(Json(PostResponseBody { session_id }))
}
