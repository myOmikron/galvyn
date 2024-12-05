//! The http part of the webserver

pub mod common;
pub mod extractors;
pub mod handler_frontend;
pub mod middlewares;
pub mod server;
mod session_keys;

/// The key for accessing the user in the session
pub const SESSION_USER: &str = "user";
/// The key for accessing and storing the data required for a secure OIDC request
///
/// I.e. csrf token, some nonce, etc.
pub const SESSION_OIDC_REQUEST: &str = "oidc_request";
