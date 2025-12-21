use std::sync::{Arc, OnceLock, RwLock};

use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHashString};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::error::Error;

pub struct Session {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub type SessionId = Uuid;
type GlobalSession = Arc<RwLock<Option<Session>>>;

pub struct AuthService {
    password_hash_str: PasswordHashString,
    session_duration: Duration,
    session_cooldown: Duration,
}

impl AuthService {
    pub fn new(
        password_hash_str: PasswordHashString,
        session_duration: Duration,
        session_cooldown: Duration,
    ) -> Self {
        AuthService {
            password_hash_str,
            session_duration,
            session_cooldown,
        }
    }

    fn global_session() -> &'static GlobalSession {
        static ACTIVE_SESSION: OnceLock<GlobalSession> = OnceLock::new();
        ACTIVE_SESSION.get_or_init(|| Arc::new(RwLock::new(None)))
    }

    fn is_session_in_cooldown(&self, session: &Session) -> bool {
        let now = Utc::now();
        now < session.created_at + self.session_cooldown
    }

    pub fn validate_session(&self, session_id: SessionId) -> Result<(), Error> {
        let mut global_session_lock = Self::global_session().write().map_err(|_| {
            log::error!("Failed to acquire write lock for global session");
            Error::UnexpectedError
        })?;

        let Some(global_session) = global_session_lock.as_ref() else {
            return Err(Error::Unauthenticated);
        };

        if session_id != global_session.id {
            return Err(Error::Unauthenticated);
        }

        let now = Utc::now();
        if now >= global_session.expires_at {
            *global_session_lock = None;
            return Err(Error::SessionExpired);
        }

        Ok(())
    }

    pub fn sign_in(&self, password: String) -> Result<SessionId, Error> {
        let expected_hash = self.password_hash_str.password_hash();
        Argon2::default()
            .verify_password(password.as_bytes(), &expected_hash)
            .map_err(|_| Error::IncorrectPassword)?;

        let mut global_session = Self::global_session().write().map_err(|_| {
            log::error!("Failed to acquire write lock for global session");
            Error::UnexpectedError
        })?;

        if global_session
            .as_ref()
            .is_some_and(|session| self.is_session_in_cooldown(session))
        {
            return Err(Error::SessionCooldown);
        }

        let session_id = Uuid::new_v4();
        let created_at = Utc::now();
        let expires_at = created_at + self.session_duration;
        let session = Session {
            id: session_id,
            created_at,
            expires_at,
        };

        *global_session = Some(session);

        Ok(session_id)
    }

    pub fn sign_out(&self) -> Result<(), Error> {
        let mut global_session_lock = Self::global_session().write().map_err(|_| {
            log::error!("Failed to acquire write lock for global session");
            Error::UnexpectedError
        })?;

        *global_session_lock = None;

        Ok(())
    }
}
