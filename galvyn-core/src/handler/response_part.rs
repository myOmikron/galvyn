use axum::http::HeaderName;

use crate::macro_utils::type_metadata::HasMetadata;
use crate::macro_utils::type_metadata::ShouldHaveMetadata;

/// Describes the behaviour of a type implementing [`IntoResponseParts`](axum::response::IntoResponseParts)
pub trait ResponsePart: ShouldBeResponsePart {
    fn header() -> Vec<HeaderName>;
}

pub trait ShouldBeResponsePart {}

#[derive(Clone, Debug)]
pub struct ResponsePartMetadata {
    pub header: fn() -> Vec<HeaderName>,
}

impl<T: ShouldBeResponsePart> ShouldHaveMetadata<ResponsePartMetadata> for T {}
impl<T: ResponsePart> HasMetadata<ResponsePartMetadata> for T {
    fn metadata() -> ResponsePartMetadata {
        ResponsePartMetadata { header: T::header }
    }
}
