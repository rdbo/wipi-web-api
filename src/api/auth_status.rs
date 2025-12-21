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
    status: String,
}

pub async fn post(
    _user_session: UserSession, // Force an authenticated user
    Extension(auth_service): Extension<Arc<AuthService>>,
) -> Result<impl IntoResponse> {
    Ok(Json(PostResponseBody {
        status: "OK".to_owned(),
    }))
}
