//! The schema for the users

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::utils::checked_string::CheckedString;
use crate::utils::schemars::SchemaDateTime;
use crate::utils::secure_string::SecureString;

/// The errors of the change password request
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ChangePwFormErrors {
    /// The provided current password was invalid
    pub current_pw: bool,
}

/// The request to change the password
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChangePwRequest {
    /// The current password of the user
    pub current_pw: CheckedString<1, 255, SecureString>, // TODO replace this with "sudo" mode
    /// The password that should be set
    pub new_pw: CheckedString<1, 255, SecureString>,
}

/// The request to create a new TOTP key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTotpRequest {
    /// A user defined label to identify this token
    pub label: CheckedString<1, 255>,

    /// The TOTP secret, base32 encoded (min 128 bit, max 256 bit)
    pub secret: CheckedString<32, 64, SecureString>,

    /// The current active token, for validation and sanity purposes
    pub token: CheckedString<6, 6>,
}

/// The errors of the create totp request
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
pub struct CreateTotpErrors {
    /// The `secret` was invalid
    pub secret: Option<CreateTotpSecretError>,
    /// The `token` was invalid
    pub token: bool,
}

/// Reason why `secret` in the create totp request failed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[allow(missing_docs)]
pub enum CreateTotpSecretError {
    InvalidBase32,
    InvalidRfc6238,
}

/// Simple representation of a user's totp key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimpleTotpKey {
    /// Primary key
    pub uuid: Uuid,

    /// A user defined label to identify this key
    pub label: CheckedString<1, 255>,

    /// The point in time the TOTP was added to the account
    pub created_at: SchemaDateTime,
}

/// The request to create a new WebAuthn key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateWebAuthnRequest {
    /// Should this key be usable to log in directly without a second factor?
    ///
    /// This requires an attested device.
    pub can_login: bool,

    /// A user defined label to identify this token
    pub label: CheckedString<1, 255>,
}

/// Simple representation of a user's webauthn key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimpleWebAuthnKey {
    /// Primary key
    pub uuid: Uuid,

    /// A user defined label to identify this key
    pub label: CheckedString<1, 255>,

    /// The point in time the TOTP was added to the account
    pub created_at: SchemaDateTime,

    /// Can this key be used to log in directly or is it just a 2nd factor?
    pub can_login: bool,
}

/// The full representation for the user
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FullUser {
    /// The identifier of the user
    pub uuid: Uuid,
    /// The mail of the user
    pub mail: String,
    /// Used for displaying purposes
    pub display_name: String,
    /// The preferred language of the user
    pub preferred_lang: UserLanguage,
    /// The user's permissions
    ///
    /// The `role`s of a user
    pub permissions: UserPermissions,
}

/// The possible languages of a user
#[derive(PartialEq, Debug, Copy, Clone, Deserialize, Serialize, JsonSchema)]
// Database conversion
#[derive(strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[serde(tag = "type")]
#[allow(missing_docs)]
pub enum UserLanguage {
    EN,
    DE,
}

/// The user's permissions
///
/// The `role`s of a user
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "role")]
pub enum UserPermissions {
    /// The administrator role is assigned to users who are able to administrate to platform
    ///
    /// They do not interact with business processes.
    Administrator,

    /// The internal role is assigned to our employees
    Internal,
}
