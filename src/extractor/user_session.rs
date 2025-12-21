use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use crate::{
    error::Error,
    service::{AuthService, NetlinkService, SessionId},
};
use axum::{
    RequestPartsExt,
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use uuid::Uuid;

pub struct UserSession {
    pub session_id: SessionId,
}

impl<S> FromRequestParts<S> for UserSession
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        log::trace!("Retrieving Authorization header content...");
        let bearer = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                log::trace!("Failed to retrieve Authorization header");
                Error::Unauthenticated
            })?;

        log::trace!(
            "Extracted user session token from Authorization header: {}",
            bearer.token()
        );

        let session_id: SessionId = bearer.token().try_into().map_err(|_| {
            log::trace!("Invalid Authorization header value: {}", bearer.token());
            Error::Unauthenticated
        })?;

        let auth_service = parts.extensions.get::<Arc<AuthService>>().ok_or_else(|| {
            log::error!("Failed to acquire AuthService");
            Error::UnexpectedError
        })?;

        log::trace!("Validating user session...");
        auth_service.validate_session(session_id)?;

        log::trace!("Session '{}' successfully authenticated", session_id);
        Ok(Self { session_id })
    }
}
