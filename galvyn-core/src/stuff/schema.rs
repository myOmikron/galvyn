//! Common schemas in the API

use std::borrow::Cow;

use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;
use schemars::json_schema;
use serde::Deserialize;
use serde::Serialize;
use time::Date;
use time::OffsetDateTime;
use time::Time;
use time::serde::format_description;
use uuid::Uuid;

/// The response that is sent in a case of an error the caller should present his user
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[schemars(rename = "FormErrorResponse_for_{T}")]
pub struct FormErrorResponse<T> {
    /// A constant `"Err"` used to differentiate this schema from any other "Ok" schema
    pub result: ErrorConstant,

    /// The actual error struct
    pub error: T,
}

/// Constant string `"Err"` which is documented by schemars
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[allow(missing_docs)]
pub enum ErrorConstant {
    #[default]
    Err,
}

/// The response that is sent in a case of an error the caller should report to an admin
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ApiErrorResponse {
    /// ID of the opentelemetry trace this error originated in
    #[cfg(feature = "opentelemetry")]
    pub trace_id: String,
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
#[schemars(rename = "Optional_for_{T}")]
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
#[schemars(rename = "List_for_{T}")]
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
#[schemars(rename = "Page_for_{T}")]
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
#[schemars(rename = "FormResult_for_{T}_and_{E}")]
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
    fn inline_schema() -> bool {
        <String as JsonSchema>::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
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
pub struct SchemaDate(#[serde(with = "rfc3339_date")] pub Date);

/// Implements [`JsonSchema`] for a type which is just a string with a well-defined format
///
/// ```
/// # pub struct MyDateTime;
/// formatted_string_impl!(MyDateTime, format: "date-time", example: "1970-01-01T00:00:00.0Z");
/// ```
macro_rules! formatted_string_impl {
    ($ty:ident, format: $format:literal, example: $example:literal) => {
        impl JsonSchema for $ty {
            fn inline_schema() -> bool {
                false
            }

            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($ty))
            }

            fn schema_id() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($ty))
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({
                    "type": "string",
                    "format": $format,
                    "examples": [$example],
                })
            }
        }
    };
}
formatted_string_impl!(SchemaDateTime, format: "date-time", example: "1970-01-01T00:00:00.0Z");
formatted_string_impl!(SchemaTime, format: "partial-date-time", example: "00:00:00.0");
formatted_string_impl!(SchemaDate, format: "date", example: "1970-01-01");

format_description!(rfc3339_date, Date, "[year]-[month]-[day]");

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::schema_generator::SchemaGenerator;
    use crate::schema_generator::schema_to_json;

    #[test]
    fn formatted_strings() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<SchemaDateTime>()),
            json!({"type": "string", "format": "date-time", "example": "1970-01-01T00:00:00.0Z"})
        );
        assert_eq!(
            schema_to_json(&generator.generate_refless::<SchemaTime>()),
            json!({"type": "string", "format": "partial-date-time", "example": "00:00:00.0"})
        );
        assert_eq!(
            schema_to_json(&generator.generate_refless::<SchemaDate>()),
            json!({"type": "string", "format": "date", "example": "1970-01-01"})
        );
    }

    #[test]
    fn formatted_strings_are_referenceable() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<SchemaDateTime>()),
            json!({"$ref": "#/components/schemas/SchemaDateTime"})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<SchemaTime>()),
            json!({"$ref": "#/components/schemas/SchemaTime"})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<SchemaDate>()),
            json!({"$ref": "#/components/schemas/SchemaDate"})
        );

        let definitions = generator.into_definitions();
        assert_eq!(
            schema_to_json(&definitions["SchemaDateTime"]),
            json!({"type": "string", "format": "date-time", "example": "1970-01-01T00:00:00.0Z"})
        );
        assert_eq!(
            schema_to_json(&definitions["SchemaTime"]),
            json!({"type": "string", "format": "partial-date-time", "example": "00:00:00.0"})
        );
        assert_eq!(
            schema_to_json(&definitions["SchemaDate"]),
            json!({"type": "string", "format": "date", "example": "1970-01-01"})
        );
    }

    #[test]
    fn schema_string() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<SchemaString<Uuid>>()),
            schema_to_json(&generator.generate::<String>()),
        );
        assert!(generator.into_definitions().is_empty());
    }

    #[test]
    fn error_constant() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<ErrorConstant>()),
            json!({
                "description": "Constant string `\"Err\"` which is documented by schemars",
                "type": "string",
                "enum": ["Err"]
            })
        );
    }

    #[test]
    fn form_error_response() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<FormErrorResponse<SingleLink>>()),
            json!({
                "description": "The response that is sent in a case of an error the caller should present his user",
                "type": "object",
                "required": ["result", "error"],
                "properties": {
                    "error": {
                        "description": "The actual error struct",
                        "allOf": [{"$ref": "#/components/schemas/SingleLink"}]
                    },
                    "result": {
                        "description": "A constant `\"Err\"` used to differentiate this schema from any other \"Ok\" schema",
                        "allOf": [{"$ref": "#/components/schemas/ErrorConstant"}]
                    }
                }
            })
        );

        let definitions = generator.into_definitions();
        assert_eq!(definitions.len(), 2);
        assert_eq!(
            schema_to_json(&definitions["SingleLink"]),
            json!({
                "description": "A single string representing a link wrapped in a struct",
                "type": "object",
                "required": ["link"],
                "properties": {"link": {"type": "string"}}
            })
        );
        assert_eq!(
            schema_to_json(&definitions["ErrorConstant"]),
            json!({
                "description": "Constant string `\"Err\"` which is documented by schemars",
                "type": "string",
                "enum": ["Err"]
            })
        );
    }

    #[test]
    fn form_result() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<FormResult<SingleUuid, SingleLink>>()),
            json!({
                "description": "A `Result` with a custom serialization",
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["result", "value"],
                        "properties": {
                            "result": {"type": "string", "enum": ["Ok"]},
                            "value": {"$ref": "#/components/schemas/SingleUuid"}
                        }
                    },
                    {
                        "type": "object",
                        "required": ["result", "error"],
                        "properties": {
                            "error": {"$ref": "#/components/schemas/SingleLink"},
                            "result": {"type": "string", "enum": ["Err"]}
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn optional() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<Optional<SingleUuid>>()),
            json!({
                "title": "Optional",
                "description": "A single field which might be `null`.\n\n## Rust Usage\n\nIf you want to return an `ApiJson<Option<T>>` from your handler,\nplease use `ApiJson<Optional<T>>` instead.\n\nIt simply wraps the option into a struct with a single field\nto ensure the json returned from a handler is always an object.",
                "type": "object",
                "properties": {
                    "optional": {
                        "allOf": [{"$ref": "#/components/schemas/SingleUuid"}],
                        "nullable": true
                    }
                }
            })
        );
    }

    #[test]
    fn page() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<Page<SingleUuid>>()),
            json!({
                "description": "A page of items",
                "type": "object",
                "required": ["items", "limit", "offset", "total"],
                "properties": {
                    "items": {
                        "description": "The page's items",
                        "type": "array",
                        "items": {"$ref": "#/components/schemas/SingleUuid"}
                    },
                    "limit": {
                        "description": "The limit this page was requested with",
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0
                    },
                    "offset": {
                        "description": "The offset this page was requested with",
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0
                    },
                    "total": {
                        "description": "The total number of items this page is a subset of",
                        "type": "integer",
                        "format": "int64"
                    }
                }
            })
        );
    }
}
