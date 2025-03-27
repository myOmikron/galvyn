use rorm::fields::types::{Json, MaxStr};
use rorm::prelude::ForeignModel;
use rorm::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{AttestedPasskey, Passkey};

#[derive(Model)]
pub struct Account {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    #[rorm(unique, max_length = 255)]
    pub id: String,
}

#[derive(Model)]
pub struct OidcAccount {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    pub issuer: MaxStr<255>,

    pub subject: MaxStr<255>,

    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub account: ForeignModel<Account>,
}

#[derive(Model)]
pub struct LocalAccount {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    #[rorm(max_length = 1024)]
    pub password: Option<String>,

    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub account: ForeignModel<Account>,
}

#[derive(Model)]
pub struct TotpKey {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub local_account: ForeignModel<LocalAccount>,

    #[rorm(max_length = 255)]
    pub label: String,

    #[rorm(max_length = 32)]
    pub secret: Vec<u8>,
}

#[derive(Model)]
pub struct WebAuthnKey {
    #[rorm(primary_key)]
    pub uuid: Uuid,

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
