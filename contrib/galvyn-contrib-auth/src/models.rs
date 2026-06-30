use galvyn_core::re_exports::rorm::fields::types::Json;
use galvyn_core::re_exports::rorm::prelude::ForeignModel;
use galvyn_core::re_exports::rorm::Model;
use galvyn_core::re_exports::time::OffsetDateTime;
use serde::Deserialize;
use serde::Serialize;
use webauthn_rs::prelude::AttestedPasskey;
use webauthn_rs::prelude::Passkey;

#[derive(Model)]
pub struct Account {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(max_length = 255)]
    pub username: String,

    #[rorm(max_length = 255)]
    pub first_name: Option<String>,

    #[rorm(max_length = 255)]
    pub last_name: Option<String>,

    #[rorm(max_length = 255)]
    pub email: Option<String>,

    pub is_active: bool,

    pub is_superuser: bool,

    pub last_login: Option<OffsetDateTime>,

    #[rorm(auto_create_time)]
    pub created_at: OffsetDateTime,
}

#[allow(non_upper_case_globals)]
pub const AccountOidcLogin: __GenericOidcLogin_ValueSpaceImpl<Account> =
    GenericOidcLogin::<Account>;
pub type AccountOidcLogin = GenericOidcLogin<Account>;
rorm::register_model!(AccountOidcLogin);

#[derive(Model)]
#[rorm(experimental_generics)]
pub struct GenericOidcLogin<Account: Model> {
    #[rorm(id)]
    pub pk: i64,

    #[rorm(max_length = 255)]
    pub sub: String,

    #[rorm(max_length = 255)]
    pub iss: String,

    pub account: ForeignModel<Account>,
}
