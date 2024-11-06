//! The [`HandlerArgument`] trait, its implementations and utilises for implementing it.

mod impls;
mod utils;

use indexmap::IndexMap;
use mime::Mime;
use openapiv3::MediaType;
use openapiv3::Parameter;
use openapiv3::ReferenceOr;
use openapiv3::RequestBody;
use openapiv3::Schema;

use crate::internals::SchemaGenerator;
use crate::type_metadata::{HasMetadata, ShouldHaveMetadata};

/// Marker trait
pub trait ShouldBeHandlerArgument {}

/// A type used as argument to a handler which can be described
/// by a [request body object](https://spec.openapis.org/oas/v3.0.3#request-body-object)
/// or a [parameter object](https://spec.openapis.org/oas/v3.0.3#parameter-object)
///
/// This type should be implemented by everything which implements
/// [`FromRequest`](::axum::extract::FromRequest) / [`FromRequestParts`](::axum::extract::FromRequestParts) when using [axum](::axum) or
/// [`FromRequest`] when using [actix]
pub trait HandlerArgument: ShouldBeHandlerArgument {
    /// Get the [request body object](https://spec.openapis.org/oas/v3.0.3#request-body-object) describing `Self`
    ///
    /// Should return `None` if `Self` doesn't consume the request body
    fn request_body(_gen: &mut SchemaGenerator) -> Option<RequestBody> {
        None
    }

    /// Get the [parameter objects](https://spec.openapis.org/oas/v3.0.3#parameter-object) describing `Self`
    ///
    /// Should return an empty `Vec` if `Self` doesn't parse any parameters
    fn parameters(_gen: &mut SchemaGenerator, _path: &[&str]) -> Vec<Parameter> {
        Vec::new()
    }
}

/// Struct representation of a [`HandlerArgument`]
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct HandlerArgumentFns {
    pub(crate) request_body: fn(&mut SchemaGenerator) -> Option<RequestBody>,
    pub(crate) parameters: fn(&mut SchemaGenerator, &[&str]) -> Vec<Parameter>,
}

/// Helper function for building a simple [`RequestBody`]
pub fn simple_request_body(request_body: SimpleRequestBody) -> RequestBody {
    RequestBody {
        content: IndexMap::<_, _>::from_iter([(
            request_body.mime_type.to_string(),
            MediaType {
                schema: request_body.schema,
                ..Default::default()
            },
        )]),
        required: true,
        ..Default::default()
    }
}

/// Describes the response for a specific status code
pub struct SimpleRequestBody {
    /// The request body's mime type
    pub mime_type: Mime,

    /// Optional schema
    pub schema: Option<ReferenceOr<Schema>>,
}

impl<T: ShouldBeHandlerArgument> ShouldHaveMetadata<HandlerArgumentFns> for T {}
impl<T: HandlerArgument> HasMetadata<HandlerArgumentFns> for T {
    fn metadata() -> HandlerArgumentFns {
        HandlerArgumentFns {
            request_body: T::request_body,
            parameters: T::parameters,
        }
    }
}
