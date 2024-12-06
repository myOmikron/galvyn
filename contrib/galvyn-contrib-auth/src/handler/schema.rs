use openidconnect::{AuthorizationCode, CsrfToken};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetLoginFlowsRequest {
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "provider")]
pub enum GetLoginFlowsResponse {
    Oidc(OidcLoginFlow),
    Local(LocalLoginFlow),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OidcLoginFlow {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalLoginFlow {
    pub password: bool,
    pub webauthn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[allow(missing_docs)]
pub struct FinishLoginOidcRequest {
    #[schemars(with = "String")]
    pub code: AuthorizationCode,
    #[schemars(with = "String")]
    pub state: CsrfToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginLocalWebauthnRequest {
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginLocalPasswordRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyCredential(pub webauthn_rs::prelude::PublicKeyCredential);
impl JsonSchema for PublicKeyCredential {
    fn schema_name() -> String {
        "PublicKeyCredential".to_owned()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        Cow::Borrowed(concat!(module_path!(), "::", "PublicKeyCredential"))
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}
