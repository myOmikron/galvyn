use crate::handler::request_part::{RequestPart, ShouldBeRequestPart};
use crate::macro_utils::type_metadata::{HasMetadata, ShouldHaveMetadata};
use crate::schema_generator::SchemaGenerator;
use mime::Mime;
use schemars::schema::Schema;

/// Describes the behaviour of a type implementing [`FromRequest`](axum::extract::FromRequest)
pub trait RequestBody: ShouldBeRequestBody {
    fn body(_generator: &mut SchemaGenerator) -> (Mime, Option<Schema>);
}

pub trait ShouldBeRequestBody {}

#[derive(Clone, Debug)]
pub struct RequestBodyMetadata {
    pub body: fn(&mut SchemaGenerator) -> (Mime, Option<Schema>),
}

impl<T: ShouldBeRequestBody> ShouldHaveMetadata<RequestBodyMetadata> for T {}
impl<T: RequestBody> HasMetadata<RequestBodyMetadata> for T {
    fn metadata() -> RequestBodyMetadata {
        RequestBodyMetadata { body: T::body }
    }
}

impl<T: ShouldBeRequestBody> ShouldBeRequestPart for T {}
impl<T: RequestBody> RequestPart for T {}
