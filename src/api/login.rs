use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{api::Result, extractor::RouterClient, service::AuthService};

#[derive(Deserialize)]
pub struct PostRequestBody {
    password: String,
}

#[derive(Serialize)]
pub struct PostResponseBody {
    auth_token: String,
}

pub async fn post(
    router_client: RouterClient,
    Extension(auth_service): Extension<Arc<AuthService>>,
    Json(PostRequestBody { password }): Json<PostRequestBody>,
) -> Result<impl IntoResponse> {
    log::info!(
        "Router client '{}' (MAC: {}) attemping sign in...",
        router_client.ip_address,
        router_client.mac_address
    );
    let session_id = auth_service.sign_in(password)?.to_string();
    log::info!("New session created: {}", session_id);

    Ok(Json(PostResponseBody {
        auth_token: session_id,
    }))
}
