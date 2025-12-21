use crate::error::Error;

pub mod auth_status;
pub mod login;
pub mod logout;

// Result for all endpoints that can fail
pub type Result<T> = core::result::Result<T, Error>;
