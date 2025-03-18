//! Common schemas in the API

use std::borrow::Cow;

use schemars::JsonSchema;
use schemars::JsonSchema_repr;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;
use time::Date;
use time::OffsetDateTime;
use time::Time;
use uuid::Uuid;

/// The Status code that are returned throughout the API
#[derive(Debug, Clone, Copy, Deserialize_repr, Serialize_repr, JsonSchema_repr)]
#[repr(u16)]
#[allow(missing_docs)]
pub enum ApiStatusCode {
    Unauthenticated = 1000,
    BadRequest = 1001,
    InvalidJson = 1002,
    MissingPrivileges = 1003,

    InternalServerError = 2000,
}

/// The response that is sent in a case of an error
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[allow(missing_docs)]
pub struct ApiErrorResponse {
    /// The Status code for the error.
    ///
    /// Important: Does not match http status codes
    pub status_code: ApiStatusCode,
    /// A human-readable error message.
    ///
    /// May be used for displaying purposes
    pub message: String,
}

/// A type without any runtime value
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub enum Never {}

/// A single uuid wrapped in a struct
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub struct SingleUuid {
    #[allow(missing_docs)]
    pub uuid: Uuid,
}

/// A single string representing a link wrapped in a struct
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SingleLink {
    #[allow(missing_docs)]
    pub link: String,
}

/// # Optional
/// A single field which might be `null`.
///
/// ## Rust Usage
///
/// If you want to return an `ApiJson<Option<T>>` from your handler,
/// please use `ApiJson<Optional<T>>` instead.
///
/// It simply wraps the option into a struct with a single field
/// to ensure the json returned from a handler is always an object.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub struct Optional<T> {
    #[allow(missing_docs)]
    pub optional: Option<T>,
}
impl<T> Optional<T> {
    /// Shorthand for `Optional { optional: Some(value) }`
    pub fn some(value: T) -> Self {
        Self {
            optional: Some(value),
        }
    }

    /// Shorthand for `Optional { optional: None }`
    pub fn none() -> Self {
        Self { optional: None }
    }
}
/// # List
/// A single field which is an array.
///
/// ## Rust Usage
///
/// If you want to return an `ApiJson<Vec<T>>` from your handler,
/// please use `ApiJson<List<T>>` instead.
///
/// It simply wraps the vector into a struct with a single field
/// to ensure the json returned from a handler is always an object.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct List<T> {
    #[allow(missing_docs)]
    pub list: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPageRequest {
    /// The limit this page was requested with
    pub limit: u64,

    /// The offset this page was requested with
    pub offset: u64,
}

/// A page of items
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
    /// The page's items
    pub items: Vec<T>,

    /// The limit this page was requested with
    pub limit: u64,

    /// The offset this page was requested with
    pub offset: u64,

    /// The total number of items this page is a subset of
    pub total: i64,
}

/// A `Result` with a custom serialization
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "result")]
#[allow(missing_docs)]
pub enum FormResult<T, E> {
    Ok { value: T },
    Err { error: E },
}
impl<T, E> FormResult<T, E> {
    /// Convenience function to construct a `FormResult::Ok`
    pub fn ok(value: T) -> Self {
        Self::Ok { value }
    }

    /// Convenience function to construct a `FormResult::Err`
    pub fn err(error: E) -> Self {
        Self::Err { error }
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

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <String as JsonSchema>::json_schema(generator)
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

/// Implements [`JsonSchema`] for a type which is just a string with a well-defined format
///
/// ```
/// # pub struct MyDateTime;
/// formatted_string_impl!(MyDateTime, format: "date-time", example: "1970-01-01T00:00:00.0Z");
/// ```
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
