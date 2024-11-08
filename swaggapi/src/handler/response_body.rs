use crate::type_metadata::{HasMetadata, ShouldHaveMetadata};

/// Describes the behaviour of a type implementing [`IntoResponse`](axum::response::IntoResponse)
pub trait ResponseBody: ShouldBeResponseBody {}

pub trait ShouldBeResponseBody {}

#[derive(Clone, Debug)]
pub struct ResponseBodyMetadata {}

impl<T: ShouldBeResponseBody> ShouldHaveMetadata<ResponseBodyMetadata> for T {}
impl<T: ResponseBody> HasMetadata<ResponseBodyMetadata> for T {
    fn metadata() -> ResponseBodyMetadata {
        ResponseBodyMetadata {}
    }
}
