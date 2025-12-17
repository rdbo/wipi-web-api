use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;

use crate::service::{AuthError, AuthService};

#[derive(Deserialize)]
pub struct PostRequestBody {
    password: String,
}

pub async fn post(
    State(auth_service): State<Arc<AuthService>>,
    Json(PostRequestBody { password }): Json<PostRequestBody>,
) -> Result<impl IntoResponse, AuthError> {
    auth_service.try_login(password)?;

    Ok("OK".to_owned())
}
