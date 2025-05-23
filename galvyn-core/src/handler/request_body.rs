use mime::Mime;
use schemars::schema::Schema;

use crate::handler::request_part::RequestPart;
use crate::handler::request_part::ShouldBeRequestPart;
use crate::macro_utils::type_metadata::HasMetadata;
use crate::macro_utils::type_metadata::ShouldHaveMetadata;
use crate::schema_generator::SchemaGenerator;

/// Describes the behaviour of a type implementing [`FromRequest`](axum::extract::FromRequest)
pub trait RequestBody: ShouldBeRequestBody {
    fn query_parameters(_generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        vec![]
    }

    fn path_parameters(_generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        vec![]
    }

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
impl<T: RequestBody> RequestPart for T {
    fn path_parameters(generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        <T as RequestBody>::path_parameters(generator)
    }

    fn query_parameters(generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        <T as RequestBody>::query_parameters(generator)
    }
}
