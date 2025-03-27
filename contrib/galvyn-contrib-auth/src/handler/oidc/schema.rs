use crate::logic::oidc::OidcRequestState;
use schemars::schema::{Schema, SchemaObject};
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FinishLoginOidcRequest(pub OidcRequestState);
impl JsonSchema for FinishLoginOidcRequest {
    fn schema_name() -> String {
        "FinishLoginOidcRequest".to_owned()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(concat!(module_path!(), "::", "FinishLoginOidcRequest"))
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        let object = schema.object();
        object
            .properties
            .insert("code".to_string(), String::json_schema(generator));
        object
            .properties
            .insert("state".to_string(), String::json_schema(generator));
        object.required.insert("code".to_string());
        object.required.insert("state".to_string());
        Schema::Object(schema)
    }
}
