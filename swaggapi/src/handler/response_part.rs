use crate::type_metadata::{HasMetadata, ShouldHaveMetadata};

/// Describes the behaviour of a type implementing [`IntoResponseParts`](axum::response::IntoResponseParts)
pub trait ResponsePart: ShouldBeResponsePart {}

pub trait ShouldBeResponsePart {}

#[derive(Clone, Debug)]
pub struct ResponsePartMetadata {}

impl<T: ShouldBeResponsePart> ShouldHaveMetadata<ResponsePartMetadata> for T {}
impl<T: ResponsePart> HasMetadata<ResponsePartMetadata> for T {
    fn metadata() -> ResponsePartMetadata {
        ResponsePartMetadata {}
    }
}
