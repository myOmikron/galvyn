use galvyn::contrib::auth::MaybeAttestedPasskey;
use rorm::fields::types::Json;
use rorm::internal::field::{Field, FieldProxy};
use rorm::prelude::{ForeignModel, ForeignModelByField};
use rorm::{Model, Patch};

#[derive(Model)]
pub struct Account {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(unique, max_length = 255)]
    pub id: String,
}

#[derive(Model)]
pub struct OidcAccount {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(max_length = 255)]
    pub id: String,

    pub account: ForeignModel<Account>,
}

#[derive(Model)]
pub struct LocalAccount {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(max_length = 1024)]
    pub password: Option<String>,

    pub account: ForeignModel<Account>,
}

#[derive(Model)]
pub struct TotpKey {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub local_account: ForeignModel<LocalAccount>,

    #[rorm(max_length = 255)]
    pub label: String,

    #[rorm(max_length = 32)]
    pub secret: Vec<u8>,
}

#[derive(Model)]
pub struct WebAuthnKey {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub local_account: ForeignModel<LocalAccount>,

    #[rorm(max_length = 255)]
    pub label: String,

    pub key: Json<MaybeAttestedPasskey>,
}

pub enum AuthModels {}
impl galvyn::contrib::auth::AuthModels for AuthModels {
    type Account = Account;

    fn account_id() -> FieldProxy<impl Field<Type = String, Model = Self::Account>, Self::Account> {
        Account::F.id
    }

    fn insertable_account(id: String) -> impl Patch<Model = Self::Account> {
        #[derive(Patch)]
        #[rorm(model = "Account")]
        struct InsertableAccount {
            id: String,
        }

        InsertableAccount { id }
    }

    type OidcAccount = OidcAccount;

    fn oidc_account_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::Account as Model>::Primary>,
            Model = Self::OidcAccount,
        >,
        Self::OidcAccount,
    > {
        OidcAccount::F.account
    }

    fn oidc_account_id(
    ) -> FieldProxy<impl Field<Type = String, Model = Self::OidcAccount>, Self::OidcAccount> {
        OidcAccount::F.id
    }

    fn insertable_oidc_account(
        id: String,
        account_pk: &<<Self::Account as Model>::Primary as Field>::Type,
    ) -> impl Patch<Model = Self::OidcAccount> {
        #[derive(Patch)]
        #[rorm(model = "OidcAccount")]
        struct InsertableOidcAccount {
            id: String,
            account: ForeignModel<Account>,
        }

        InsertableOidcAccount {
            id,
            account: ForeignModelByField::Key(*account_pk),
        }
    }

    type LocalAccount = LocalAccount;

    fn local_account_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::Account as Model>::Primary>,
            Model = Self::LocalAccount,
        >,
        Self::LocalAccount,
    > {
        LocalAccount::F.account
    }

    fn local_account_password(
    ) -> FieldProxy<impl Field<Type = Option<String>, Model = Self::LocalAccount>, Self::LocalAccount>
    {
        LocalAccount::F.password
    }

    type TotpKey = TotpKey;

    fn totp_key_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::LocalAccount as Model>::Primary>,
            Model = Self::TotpKey,
        >,
        Self::TotpKey,
    > {
        TotpKey::F.local_account
    }

    type WebauthnKey = WebAuthnKey;

    fn webauthn_key_fm() -> FieldProxy<
        impl Field<
            Type = ForeignModelByField<<Self::LocalAccount as Model>::Primary>,
            Model = Self::WebauthnKey,
        >,
        Self::WebauthnKey,
    > {
        WebAuthnKey::F.local_account
    }

    fn webauthn_key_key() -> FieldProxy<
        impl Field<Type = Json<MaybeAttestedPasskey>, Model = Self::WebauthnKey>,
        Self::WebauthnKey,
    > {
        WebAuthnKey::F.key
    }
}
