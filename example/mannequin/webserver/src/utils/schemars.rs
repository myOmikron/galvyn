//! Utilities for patching missing [`JsonSchema`] implementations

use std::borrow::Cow;

use schemars::gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use time::Date;
use time::OffsetDateTime;
use time::Time;

/// Wraps a `webauthn_rs` struct to "provide" a [`JsonSchema`] implementation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebAuthnSchema<T>(pub T);
impl<T> JsonSchema for WebAuthnSchema<T> {
    fn schema_name() -> String {
        "WebAuthnSchema".to_string()
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("WebAuthnSchema")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        schema.object();
        schema.metadata().description =
            Some("This is a type provided by webauthn_rs which we lack schemas for".to_string());
        schema.metadata().examples.push(json! {{}});
        schema.into()
    }
}

/// Wrap any type to "provide" a [`JsonSchema`] implementation of [`String`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SchemaString<T>(pub T);
impl<T> JsonSchema for SchemaString<T> {
    fn is_referenceable() -> bool {
        <String as JsonSchema>::is_referenceable()
    }

    fn schema_name() -> String {
        <String as JsonSchema>::schema_name()
    }

    fn schema_id() -> Cow<'static, str> {
        <String as JsonSchema>::schema_id()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        <String as JsonSchema>::json_schema(gen)
    }
}

/// Wrapper around [`OffsetDateTime`] to add a [`JsonSchema`] impl and select RFC 3339 as serde repr.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct SchemaDateTime(#[serde(with = "time::serde::rfc3339")] pub OffsetDateTime);

/// Wrapper around [`Time`] to add a [`JsonSchema`] impl and select RFC 3339 as serde repr.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct SchemaTime(pub Time);

/// Wrapper around [`Date`] to add a [`JsonSchema`] impl and select RFC 3339 as serde repr.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct SchemaDate(pub Date);

macro_rules! formatted_string_impl {
    ($ty:ident, format: $format:literal, example: $example:literal) => {
        impl JsonSchema for $ty {
            fn is_referenceable() -> bool {
                true
            }

            fn schema_name() -> String {
                stringify!($ty).to_owned()
            }

            fn schema_id() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($ty))
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                SchemaObject {
                    instance_type: Some(InstanceType::String.into()),
                    format: Some($format.to_owned()),
                    metadata: Some(Box::new(Metadata {
                        examples: vec![json!($example)],
                        ..Default::default()
                    })),
                    ..Default::default()
                }
                .into()
            }
        }
    };
}
formatted_string_impl!(SchemaDateTime, format: "date-time", example: "1970-01-01T00:00:00.0Z");
formatted_string_impl!(SchemaTime, format: "partial-date-time", example: "00:00:00.0");
formatted_string_impl!(SchemaDate, format: "date", example: "1970-01-01");
