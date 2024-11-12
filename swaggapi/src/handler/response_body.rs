use crate::handler::response_part::{ResponsePart, ShouldBeResponsePart};
use crate::schema_generator::SchemaGenerator;
use crate::type_metadata::{HasMetadata, ShouldHaveMetadata};
use axum::http::StatusCode;
use mime::Mime;
use schemars::schema::Schema;

/// Describes the behaviour of a type implementing [`IntoResponse`](axum::response::IntoResponse)
pub trait ResponseBody: ShouldBeResponseBody {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)>;
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
impl<T: ResponseBody> ResponsePart for T {}
