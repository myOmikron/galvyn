use rorm::fields::types::Json;
use rorm::prelude::ForeignModel;
use rorm::Patch;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::models::LocalUser;
use crate::models::MaybeAttestedPasskey;
use crate::models::Role;
use crate::models::TotpKey;
use crate::models::User;
use crate::models::UserInvite;
use crate::models::WebAuthnKey;

/// Insert patch for [`User`]
#[derive(Patch)]
#[rorm(model = "User")]
pub struct UserInsert {
    /// Primary key of a user
    pub uuid: Uuid,

    /// The name that is used for displaying purposes
    pub display_name: String,

    /// The preferred language of the user
    pub preferred_lang: String,

    /// The role of a user
    ///
    /// The [`Role`] table uses its identifying string as primary key.
    /// So, there is no need to join it as all information are already returned by `ForeignModel::key()`.
    /// The value should only be used in conversions to and from [`UserRole`](crate::http::handler_frontend::users::schema::UserRole).
    pub role: ForeignModel<Role>,

    /// The mail of the user
    pub mail: String,
}

/// Insert patch for [`LocalUser`]
#[derive(Patch)]
#[rorm(model = "LocalUser")]
pub struct LocalUserInsert {
    /// Primary key of an oidc user
    pub uuid: Uuid,

    /// The reference to the user model
    pub user: ForeignModel<User>,

    /// The hashed password
    pub password: Option<String>,
}

/// Insert patch for [`TotpKey`]
#[derive(Patch)]
#[rorm(model = "TotpKey")]
pub struct TotpKeyInsert {
    /// Primary key
    pub uuid: Uuid,

    /// The reference to the user model
    pub local_user: ForeignModel<LocalUser>,

    /// A user defined label to identify this key
    pub label: String,

    /// The secret key for the totp
    pub secret: Vec<u8>,
}

/// Insert patch for [`WebAuthnKey`]
#[derive(Patch)]
#[rorm(model = "WebAuthnKey")]
pub struct WebAuthnKeyInsert {
    /// Primary key
    pub uuid: Uuid,

    /// The reference to the user model
    pub local_user: ForeignModel<LocalUser>,

    /// A user defined label to identify this key
    pub label: String,

    /// Cryptographic public key
    pub key: Json<MaybeAttestedPasskey>,
}

/// Insert patch for [`UserInvite`]
#[derive(Patch)]
#[rorm(model = "UserInvite")]
pub struct UserInviteInsert {
    /// A primary key
    pub uuid: Uuid,

    /// The `display_name` to set for the new user
    pub display_name: String,

    /// The `preferred_lang` to set for the new user
    pub preferred_lang: String,

    /// The `email` to set for the new user
    pub email: String,

    /// The `role` and associated relations to set for the new user
    pub permissions: Json<UserPermissions>,

    /// Until when is the invite valid
    pub expires_at: OffsetDateTime,
}
