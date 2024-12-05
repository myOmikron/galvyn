//! A module to provide a "secure" string implementation
//! that does not leak its content into tracing

use std::borrow::Cow;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

/// A string with a custom debug implementation
#[derive(Clone)]
pub struct SecureString(String);

impl SecureString {
    /// Constructs a new secure string
    pub const fn new(value: String) -> Self {
        Self(value)
    }
}

impl SecureString {
    /// Convert into inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Serialize for SecureString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecureString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(String::deserialize(deserializer)?))
    }
}

impl JsonSchema for SecureString {
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

impl Debug for SecureString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "-redacted-")
    }
}

impl Display for SecureString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "-redacted-")
    }
}

impl Deref for SecureString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
