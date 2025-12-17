use std::collections::HashMap;

use axum::{Json, http::StatusCode, response::IntoResponse};

pub enum Error {
    AcquireLockFailed,
    RouterClientIdentificationFailed,
    SessionCooldown,
    IncorrectPassword,
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::AcquireLockFailed | Self::RouterClientIdentificationFailed => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::SessionCooldown => StatusCode::TOO_MANY_REQUESTS,
            Self::IncorrectPassword => StatusCode::UNAUTHORIZED,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::AcquireLockFailed => "Unexpected error happened",
            Self::RouterClientIdentificationFailed => "Failed to identify the router client",
            Self::SessionCooldown => "Session creation is on cooldown",
            Self::IncorrectPassword => "Incorrect credentials",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let body = HashMap::from([("error", self.message())]);
        (self.status_code(), Json(body)).into_response()
    }
}
