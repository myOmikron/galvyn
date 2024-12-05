//! Schemas for user invites

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::http::handler_frontend::users::schema::UserLanguage;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::utils::checked_string::CheckedString;
use crate::utils::schemars::SchemaDateTime;
use crate::utils::secure_string::SecureString;

/// The response containing an invitation's details
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result")]
#[allow(missing_docs)]
pub enum GetUserInviteResponse {
    Valid { invite: SimpleUserInvite },
    NotFound,
    Expired,
}

/// The request to invite a new (local) user
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateUserInviteRequest {
    /// The mail of the user
    pub mail: CheckedString<1, 255>,

    /// The name that is used for displaying purposes
    pub display_name: CheckedString<1, 255>,

    /// The preferred language of the user
    pub preferred_lang: UserLanguage,

    /// The user's permissions
    ///
    /// Combination of a `role` and role specific `groups`
    pub permissions: UserPermissions,
}

/// The errors of the invite user request
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CreateUserInviteErrors {
    /// The `mail` is not unique
    pub mail: Option<CreateUserInviteMailError>,
}

/// Reason why `mail` in the invite user request failed
#[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
#[allow(missing_docs)]
pub enum CreateUserInviteMailError {
    AlreadyUser,
    AlreadyInvited,
}

/// An outstanding user invite
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimpleUserInvite {
    /// The primary key
    pub uuid: Uuid,

    /// The invite link
    pub link: String,

    /// The mail of the user
    pub mail: CheckedString<1, 255>,

    /// The name that is used for displaying purposes
    pub display_name: CheckedString<1, 255>,

    /// The preferred language of the user
    pub preferred_lang: UserLanguage,

    /// The user's permissions
    ///
    /// Combination of a `role` and role specific `groups`
    pub permissions: UserPermissions,

    /// Until when is the invite valid
    pub expires_at: SchemaDateTime,

    /// When was this invite created
    pub created_at: SchemaDateTime,
}

/// The request to accept an invitation by providing a password
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AcceptWithPwRequest {
    /// The password that should be set
    pub password: CheckedString<1, 0, SecureString>,
}

/// The request to accept an invitation by providing a webauthn key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AcceptWithWARequest {
    /// A user defined label to identify the login key
    pub label: CheckedString<1, 255>,
}
