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

type SessionId = Uuid;
type GlobalSession = Arc<RwLock<Option<Session>>>;

pub struct AuthService {
    password_hash_str: PasswordHashString,
    session_cooldown: Duration,
}

impl AuthService {
    pub fn new(password_hash_str: PasswordHashString, session_cooldown: Duration) -> Self {
        AuthService {
            password_hash_str,
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

    pub fn try_login(&self, password: String) -> Result<SessionId, Error> {
        let expected_hash = self.password_hash_str.password_hash();
        if Argon2::default()
            .verify_password(password.as_bytes(), &expected_hash)
            .is_err()
        {
            return Err(Error::IncorrectPassword);
        }

        let mut global_session = Self::global_session()
            .write()
            .map_err(|_| Error::AcquireLockFailed)?;

        if global_session
            .as_ref()
            .is_some_and(|session| self.is_session_in_cooldown(session))
        {
            return Err(Error::SessionCooldown);
        }

        let session_id = Uuid::new_v4();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::minutes(15);
        let session = Session {
            id: session_id,
            created_at,
            expires_at,
        };

        *global_session = Some(session);

        Ok(session_id)
    }
}
