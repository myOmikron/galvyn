//! The names and helper types of values that are stored in sessions.

use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;
use webauthn_rs::prelude::AttestedPasskeyAuthentication;
use webauthn_rs::prelude::AttestedPasskeyRegistration;
use webauthn_rs::prelude::PasskeyAuthentication;
use webauthn_rs::prelude::PasskeyRegistration;

use crate::utils::checked_string::CheckedString;

/// The key for accessing the user in the session
///
/// Value is of type `Uuid`
pub const SESSION_USER: &str = "user";

/// The key for accessing the local user in the session while it's only partially authenticated (2FA is missing)
///
/// Value is of type [`PartiallyAuthedSessionUser`]
pub const PARTIALLY_AUTHED_SESSION_USER: &str = "missing2fa";

/// The key for accessing and storing the data required for a secure OIDC request
///
/// I.e. csrf token, some nonce, etc.
///
/// Value is of type [`AuthState`](crate::http::handler_frontend::oidc::schema::AuthState)
pub const SESSION_OIDC_REQUEST: &str = "oidc_request";

/// The key for accessing and storing the data required for a webauthn authentication request
///
/// Value is of type [`WebAuthnAuthentication`]
pub const SESSION_WEBAUTHN_AUTHENTICATION: &str = "webauthn_authentication";

/// The key for accessing and storing the data required for a webauthn registration request
///
/// Value is of type [`WebAuthnRegistration`]
pub const SESSION_WEBAUTHN_REGISTRATION: &str = "webauthn_registration";

/// The key for accessing and storing the data required for accepting a user invite using webauthn
///
/// Value is of type [`WebAuthnAccept`]
pub const SESSION_WEBAUTHN_ACCEPT: &str = "webauthn_accept";

/// A local user which requires a 2nd factor to finish the login process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartiallyAuthedSessionUser {
    /// When the user tried to log in.
    ///
    /// Use to check the [`MFA_TIMEOUT`](crate::http::handler_frontend::auth::utils::MFA_TIMEOUT).
    pub timestamp: OffsetDateTime,

    /// The `LocalUser` who tries to log in
    pub local_user: Uuid,
}

/// Data required for a webauthn authentication request
///
/// Stored in a session under the key [`SESSION_WEBAUTHN_AUTHENTICATION`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAuthentication {
    /// The `LocalUser` who requested the challenge
    pub local_user: Uuid,

    /// State to check the challenge's response against
    pub state: WebAuthnAuthenticationState,
}
/// [`WebAuthnAuthentication`]'s state to check the challenge's response against
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum WebAuthnAuthenticationState {
    NotAttested(PasskeyAuthentication),
    Attested(AttestedPasskeyAuthentication),
}

/// Data required for a webauthn registration request
///
/// Stored in a session under the key [`SESSION_WEBAUTHN_REGISTRATION`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRegistration {
    /// The label to be given to the new key in registration
    pub label: CheckedString<1, 255>,

    /// The `LocalUser` who requested the challenge
    pub local_user: Uuid,

    /// State to check the challenge's response against
    pub state: WebAuthnRegistrationState,
}
/// [`WebAuthnRegistration`]'s state to check the challenge's response against
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum WebAuthnRegistrationState {
    NotAttested(PasskeyRegistration),
    Attested(AttestedPasskeyRegistration),
}

/// Data required for accepting a user invite using webauthn
///
/// Stored in a session under the key [`SESSION_WEBAUTHN_ACCEPT`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAccept {
    /// The label to be given to the new key in registration
    pub label: CheckedString<1, 255>,

    /// The uuid for the `User` to be created.
    ///
    /// This uuid has to exist before the model does because it is an argument to
    /// [`WebAuthn::start_attested_passkey_registration`](webauthn_rs::Webauthn::start_attested_passkey_registration).
    pub user_uuid: Uuid,

    /// The uuid for the `UserInvite` to be accepted
    pub invite_uuid: Uuid,

    /// State to check the challenge's response against
    pub state: AttestedPasskeyRegistration,
}
