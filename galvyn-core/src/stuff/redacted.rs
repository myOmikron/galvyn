//! Newtype to hide a value from tracing
//!
//! This is useful for things like passwords and other secrets

use std::borrow::Cow;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

use schemars::JsonSchema;
use schemars::SchemaGenerator;
use schemars::schema::Schema;
use serde::Deserialize;
use serde::Serialize;

/// Wraps any type to hide its content from tracing
///
/// This is useful for things like passwords and other secrets
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Redacted<T>(pub T);

impl<T: JsonSchema> JsonSchema for Redacted<T> {
    fn is_referenceable() -> bool {
        <T as JsonSchema>::is_referenceable()
    }

    fn schema_name() -> String {
        <T as JsonSchema>::schema_name()
    }

    fn schema_id() -> Cow<'static, str> {
        <T as JsonSchema>::schema_id()
    }

    fn json_schema(sg: &mut SchemaGenerator) -> Schema {
        <T as JsonSchema>::json_schema(sg)
    }
}

impl<T> Debug for Redacted<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Redacted").finish_non_exhaustive()
    }
}

impl<T> Display for Redacted<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "-redacted-")
    }
}
