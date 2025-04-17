use galvyn_core::re_exports::uuid::Uuid;
use rorm::Model;
use rorm::fields::types::MaxStr;

/// A registered application which may perform oauth requests
#[derive(Model)]
pub struct GalvynOauthClient {
    /// The primary key as well as oauth's `client_id`
    #[rorm(primary_key)]
    pub uuid: Uuid,

    /// A name to show the user when asking for permissions
    pub name: MaxStr<255>,

    /// oauth's `client_secret` to compare with in the `/token` request
    pub secret: MaxStr<255>,

    /// oauth's `redirect_uri` to compare with in the initial `/auth` request
    pub redirect_uri: MaxStr<255>,
}
