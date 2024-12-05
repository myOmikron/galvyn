//! All user related models are defined here

use rorm::field;
use rorm::fields::types::Json;
use rorm::prelude::BackRef;
use rorm::prelude::ForeignModel;
use rorm::Model;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;
use webauthn_rs::prelude::AttestedPasskey;
use webauthn_rs::prelude::Passkey;

use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::models::Role;

mod impls;
mod patches;

pub use self::impls::*;
pub use self::patches::*;

/// The representation of a user
#[derive(Model, Clone)]
pub struct User {
    /// Primary key of a user
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The name that is used for displaying purposes
    #[rorm(max_length = 255)]
    pub display_name: String,

    /// The preferred language of the user
    #[rorm(max_length = 255)]
    pub preferred_lang: String,

    /// The role of a user
    ///
    /// The [`Role`] table uses its identifying string as primary key.
    /// So, there is no need to join it as all information are already returned by `ForeignModel::key()`.
    /// The value should only be used in conversions to and from [`UserRole`](crate::http::handler_frontend::users::schema::UserRole).
    #[rorm(on_update = "Cascade")]
    pub role: ForeignModel<Role>,

    /// The mail of the user
    #[rorm(max_length = 255, unique)]
    pub mail: String,
}

/// A user that is identified though an IDM server
#[derive(Model)]
pub struct OidcUser {
    /// Primary key of an oidc user
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The reference to the user model
    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub user: ForeignModel<User>,

    /// The ID provided by the openid server
    #[rorm(max_length = 255)]
    pub oidc_id: String,
}

/// A locally authenticated user
#[derive(Model)]
pub struct LocalUser {
    /// Primary key of an oidc user
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The reference to the user model
    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub user: ForeignModel<User>,

    /// The hashed password, if any
    ///
    /// Use `verify_pw` and `hash_pw` (from [`utils::hashing`](crate::utils::hashing)) to read and write this field.
    #[rorm(max_length = 1024)]
    pub password: Option<String>,

    /// TOTP keys registers for this user
    pub totp: BackRef<field!(TotpKey::F.local_user)>,

    /// WebAuthn keys registered for this user
    pub webauthn: BackRef<field!(WebAuthnKey::F.local_user)>,
}

/// A TOTP key registered with an authenticator app by the user
#[derive(Model)]
pub struct TotpKey {
    /// Primary key
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The reference to the user model
    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub local_user: ForeignModel<LocalUser>,

    /// A user defined label to identify this key
    #[rorm(max_length = 255)]
    pub label: String,

    /// The secret key for the totp
    #[rorm(max_length = 32)]
    pub secret: Vec<u8>,

    /// The point in time the TOTP was added to the account
    #[rorm(auto_create_time)]
    pub created_at: OffsetDateTime,
}

/// A WebAuthn key registered by the user.
///
/// If the key is `user_verified`, it can be used a password-less login.
#[derive(Model)]
pub struct WebAuthnKey {
    /// Primary key
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The reference to the user model
    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub local_user: ForeignModel<LocalUser>,

    /// A user defined label to identify this key
    #[rorm(max_length = 255)]
    pub label: String,

    /// Cryptographic public key
    pub key: Json<MaybeAttestedPasskey>,

    /// The point in time the TOTP was added to the account
    #[rorm(auto_create_time)]
    pub created_at: OffsetDateTime,
}

/// The value of [`WebAuthnKey`]`.key`
///
/// It is a [`webauthn_rs::Passkey`] which preserves whether he is attested or not.
#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MaybeAttestedPasskey {
    NotAttested(Passkey),
    Attested(AttestedPasskey),
}

/// M2M model between [`User`] and [`InternalGroup`]
#[derive(Model, Clone)]
pub struct UserGroups {
    /// A primary key
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The user
    #[rorm(on_update = "Cascade", on_delete = "Cascade")]
    pub user: ForeignModel<User>,
}

/// An outstanding invite link for a new local user to register himself
#[derive(Model)]
pub struct UserInvite {
    /// A primary key
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// The `display_name` to set for the new user
    #[rorm(max_length = 255)]
    pub display_name: String,

    /// The `preferred_lang` to set for the new user
    #[rorm(max_length = 255)]
    pub preferred_lang: String,

    /// The `email` to set for the new user
    #[rorm(max_length = 255)]
    pub email: String,

    /// The `role` and associated relations to set for the new user
    pub permissions: Json<UserPermissions>,

    /// Until when is the invite valid
    pub expires_at: OffsetDateTime,

    /// When was this invite created
    #[rorm(auto_create_time)]
    pub created_at: OffsetDateTime,
}
