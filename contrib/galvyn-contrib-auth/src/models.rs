use rorm::fields::types::Json;
use rorm::prelude::ForeignModel;
use rorm::Model;
use serde::Deserialize;
use serde::Serialize;
use webauthn_rs::prelude::AttestedPasskey;
use webauthn_rs::prelude::Passkey;

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

#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MaybeAttestedPasskey {
    NotAttested(Passkey),
    Attested(AttestedPasskey),
}
