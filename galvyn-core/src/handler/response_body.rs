use axum::http::HeaderName;
use axum::http::StatusCode;
use mime::Mime;
use schemars::schema::Schema;

use crate::handler::response_part::ResponsePart;
use crate::handler::response_part::ShouldBeResponsePart;
use crate::macro_utils::type_metadata::HasMetadata;
use crate::macro_utils::type_metadata::ShouldHaveMetadata;
use crate::schema_generator::SchemaGenerator;

/// Describes the behaviour of a type implementing [`IntoResponse`](axum::response::IntoResponse)
pub trait ResponseBody: ShouldBeResponseBody {
    fn header() -> Vec<HeaderName> {
        vec![]
    }
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)>;
}

pub trait ShouldBeResponseBody {}

#[derive(Clone, Debug)]
pub struct ResponseBodyMetadata {
    pub body: fn(&mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)>,
}

impl<T: ShouldBeResponseBody> ShouldHaveMetadata<ResponseBodyMetadata> for T {}
impl<T: ResponseBody> HasMetadata<ResponseBodyMetadata> for T {
    fn metadata() -> ResponseBodyMetadata {
        ResponseBodyMetadata { body: T::body }
    }
}

impl<T: ShouldBeResponseBody> ShouldBeResponsePart for T {}
impl<T: ResponseBody> ResponsePart for T {
    fn header() -> Vec<HeaderName> {
        <T as ResponseBody>::header()
    }
}
