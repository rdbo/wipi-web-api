use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{error::Error, service::AuthService};

#[derive(Deserialize)]
pub struct PostRequestBody {
    password: String,
}

#[derive(Serialize)]
pub struct PostResponseBody {
    session_id: String,
}

pub async fn post(
    State(auth_service): State<Arc<AuthService>>,
    Json(PostRequestBody { password }): Json<PostRequestBody>,
) -> Result<impl IntoResponse, Error> {
    let session_id = auth_service.try_login(password)?.to_string();

    Ok(Json(PostResponseBody { session_id }))
}
