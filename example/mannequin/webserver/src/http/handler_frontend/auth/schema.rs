//! The schema for local authentication

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::utils::checked_string::CheckedString;
use crate::utils::secure_string::SecureString;

/// The request for to retrieve a user's possible login flows
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginFlowsRequest {
    /// The mail whose login flows to query
    pub mail: CheckedString<1, 255>,
}

/// Flags indicating which login flows are supported by an email's account.
///
/// If `oidc` is `true`, the others have to be `false`.
/// If `oidc` is `false`, at least one of the others has to be `true`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SupportedLoginFlows {
    /// The mail the login flows are for
    pub mail: CheckedString<1, 255>,

    /// Is this email authenticated through OpenId Connect?
    pub oidc: bool,

    /// Does this email support password login?
    pub password: bool,

    /// Does this email support password-less login through a security key?
    pub key: bool,
}

/// The request for local login using webauthn
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginWebauthnRequest {
    /// The mail that is used for logging in
    pub mail: CheckedString<1, 255>,
}

/// The errors of the login webauthn request
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LoginWebauthnErrors {
    /// This mail doesn't correspond to a local user
    pub mail: bool,
}

/// The request for local login using a password
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginPasswordRequest {
    /// The mail that is used for logging in
    pub mail: CheckedString<1, 255>,
    /// The password for the user
    pub password: CheckedString<1, 255, SecureString>,
}

/// The errors of the login password request
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LoginPasswordErrors {
    /// This mail doesn't correspond to a local user
    pub mail: bool,
    /// The password was invalid
    pub password: bool,
}

/// The MFA options available to a user after login
#[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MFA {
    /// The user has the option of using a TOTP key
    pub has_totp: bool,
    /// The user has the option of using a webauthn key
    pub has_webauthn: bool,
}

/// The response for local login using a password
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "res")]
pub enum LoginPasswordResponse {
    /// Needs further 2FA request that will perform authentication.
    NeedMFA {
        /// The MFA options available to a user
        mfa: MFA,
    },
    // VERY IMPORTANT!!!!
    // DO NOT MOVE THIS VALUE ABOVE Need2FA AS THE TYPESCRIPT
    // GENERATOR TRIES TO UNPACK THE VARIANTS IN ORDER AND PICKS THE FIRST MATCHING ONE
    /// Fully authenticated, session was set.
    Finished,
}

/// The request to verify a password login using an TOTP key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VerifyTotpRequest {
    /// The 6-digit TOTP token
    pub token: CheckedString<6, 6>,
}

/// The result when authenticating with a registered webauthn key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result")]
#[allow(missing_docs)] // typescript generator can't handle them
pub enum WebAuthnAuthenticateResult {
    Ok,
    Err,
}
