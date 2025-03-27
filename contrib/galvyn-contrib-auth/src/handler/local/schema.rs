use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use webauthn_rs::prelude::{PublicKeyCredential, RequestChallengeResponse};

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
pub struct LoginLocalWebauthnResponse(pub RequestChallengeResponse);
impl JsonSchema for LoginLocalWebauthnResponse {
    fn schema_name() -> String {
        "LoginLocalWebauthnResponse".to_owned()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(concat!(module_path!(), "::", "LoginLocalWebauthnResponse"))
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishLoginLocalWebauthnRequest(pub PublicKeyCredential);
impl JsonSchema for FinishLoginLocalWebauthnRequest {
    fn schema_name() -> String {
        "FinishLoginLocalWebauthnRequest".to_owned()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(concat!(
            module_path!(),
            "::",
            "FinishLoginLocalWebauthnRequest"
        ))
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}
