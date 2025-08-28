use axum::http::HeaderName;
use axum::http::StatusCode;
use mime::Mime;
use schemars::schema::Schema;

use crate::handler::context::EndpointContext;
use crate::handler::response_part::ResponsePart;
use crate::handler::response_part::ShouldBeResponsePart;
use crate::macro_utils::type_metadata::HasMetadata;
use crate::macro_utils::type_metadata::ShouldHaveMetadata;

/// Describes the behaviour of a type implementing [`IntoResponse`](axum::response::IntoResponse)
pub trait ResponseBody: ShouldBeResponseBody {
    fn header() -> Vec<HeaderName> {
        vec![]
    }

    #[allow(
        clippy::type_complexity,
        reason = "Type should be self-explanatory and indirection would add noise"
    )]
    fn body(_generator: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)>;
}

pub trait ShouldBeResponseBody {}

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct ResponseBodyMetadata {
    pub body: fn(&mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)>,
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
