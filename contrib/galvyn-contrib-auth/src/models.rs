use rorm::fields::traits::FieldEq;
use rorm::fields::types::Json;
use rorm::internal::field::as_db_type::AsDbType;
use rorm::internal::field::decoder::FieldDecoder;
use rorm::internal::field::{Field, FieldProxy};
use rorm::model::GetField;
use rorm::prelude::ForeignModelByField;
use rorm::{FieldAccess, Model};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::{AttestedPasskey, Passkey};

pub trait AuthModels: Send + 'static {
    type Session: Model<Primary: Field<Type = String>>;
    /// The foreign model field for a session's potentially logged-in account
    fn session_account() -> FieldProxy<
        impl Field<
            Type = Option<ForeignModelByField<<Self::Account as Model>::Primary>>,
            Model = Self::Session,
        >,
        Self::Session,
    >;

    type Account: Model<Primary: Field<Type: AsDbType>>
        + GetField<<Self::Account as Model>::Primary>;
    /// The account's identifier field
    ///
    /// The identifier is a string used by users to identify their accounts.
    /// It must be unique and suitable to be known to and remembered by a user.
    fn account_id() -> FieldProxy<impl Field<Type = String, Model = Self::Account>, Self::Account>;
    /// The account's primary key field
    ///
    /// The primary key MAY be the same as the identifier.
    /// An application SHOULD use a different field.
    fn account_pk() -> FieldProxy<<Self::Account as Model>::Primary, Self::Account> {
        FieldProxy::new()
    }

    type OidcAccount: Model;
    fn oidc_account_pk() -> FieldProxy<<Self::OidcAccount as Model>::Primary, Self::OidcAccount> {
        FieldProxy::new()
    }
    /// The foreign model field of `OidcAccount` pointing to `Account`
    fn oidc_account_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::Account as Model>::Primary>,
            Model = Self::OidcAccount,
        >,
        Self::OidcAccount,
    >;
    fn oidc_account_id(
    ) -> FieldProxy<impl Field<Type = String, Model = Self::OidcAccount>, Self::OidcAccount>;

    type LocalAccount: Model<Primary: Field<Type: AsDbType>>
        + GetField<<Self::LocalAccount as Model>::Primary>;
    fn local_account_pk() -> FieldProxy<<Self::LocalAccount as Model>::Primary, Self::LocalAccount>
    {
        FieldProxy::new()
    }
    /// The foreign model field of `LocalAccount` pointing to `Account`
    fn local_account_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::Account as Model>::Primary>,
            Model = Self::LocalAccount,
        >,
        Self::LocalAccount,
    >;
    fn local_account_password(
    ) -> FieldProxy<impl Field<Type = Option<String>, Model = Self::LocalAccount>, Self::LocalAccount>;

    type TotpKey: Model;
    fn totp_key_pk() -> FieldProxy<<Self::TotpKey as Model>::Primary, Self::TotpKey> {
        FieldProxy::new()
    }
    fn totp_key_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::LocalAccount as Model>::Primary>,
            Model = Self::TotpKey,
        >,
        Self::TotpKey,
    >;

    type WebauthnKey: Model;
    fn webauthn_key_pk() -> FieldProxy<<Self::WebauthnKey as Model>::Primary, Self::WebauthnKey> {
        FieldProxy::new()
    }
    fn webauthn_key_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::LocalAccount as Model>::Primary>,
            Model = Self::WebauthnKey,
        >,
        Self::WebauthnKey,
    >;
    fn webauthn_key_key() -> FieldProxy<
        impl Field<Type = Json<MaybeAttestedPasskey>, Model = Self::WebauthnKey>,
        Self::WebauthnKey,
    >;
}

#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MaybeAttestedPasskey {
    NotAttested(Passkey),
    Attested(AttestedPasskey),
}
