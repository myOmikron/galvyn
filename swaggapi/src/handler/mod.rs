use self::request_body::RequestBodyMetadata;
use self::request_part::RequestPartMetadata;
use self::response_body::ResponseBodyMetadata;
use self::response_part::ResponsePartMetadata;
use crate::type_metadata::{HasMetadata, ShouldHaveMetadata};
use axum::http::response::Parts;
use axum::http::{Method, Response, StatusCode};
use axum::routing::MethodRouter;

mod impls;
pub mod request_body;
pub mod request_part;
pub mod response_body;
pub mod response_part;

/// A function handling a web request
///
/// This trait should not be implemented by hand.
/// Instead, the [`#[handler]`](crate::handler) macro (and its siblings)
/// annotate a function and implements this trait on them[^clarification].
///
/// [^clarification]: Currently, there is no way to implement a trait on a specific function,
///     because a function's type can't be named.
///     The macro creates a zero-sized struct which shadows the function.
pub trait GalvynHandler {
    /// Meta information about a [`GalvynHandler`] gathered by the [`#[handler]`](crate::handler) macro
    fn meta(&self) -> HandlerMeta;

    /// The actual function stored in an axum specific format
    fn method_router(&self) -> MethodRouter;
}

/// Meta information about a [`GalvynHandler`] gathered by the [`#[handler]`](crate::handler) macro
#[derive(Clone, Debug)]
pub struct HandlerMeta {
    /// The http method the handler handles
    pub method: Method,

    /// The handler's path
    pub path: &'static str,

    /// `true` if `#[deprecated]` is present
    pub deprecated: bool,

    /// Set by macro if `#[doc = "..."]` (i.e. a doc comment) is present
    pub doc: &'static [&'static str],

    /// The handler's identifier
    pub ident: &'static str,

    /// Tags set through `#[operation(..., tags(...))]`
    pub tags: &'static [&'static str],

    pub request_parts: Vec<RequestPartMetadata>,

    pub request_body: Option<RequestBodyMetadata>,

    pub response_modifier: Option<ResponseModifier>,

    pub response_parts: Vec<ResponsePartMetadata>,

    pub response_body: Option<ResponseBodyMetadata>,
}

#[derive(Clone, Debug)]
pub enum ResponseModifier {
    StatusCode,
    Parts,
    Response,
}

impl<T: HasMetadata<ResponseModifier>> ShouldHaveMetadata<ResponseModifier> for T {}
impl HasMetadata<ResponseModifier> for StatusCode {
    fn metadata() -> ResponseModifier {
        ResponseModifier::StatusCode
    }
}
impl HasMetadata<ResponseModifier> for Parts {
    fn metadata() -> ResponseModifier {
        ResponseModifier::StatusCode
    }
}

impl HasMetadata<ResponseModifier> for Response<()> {
    fn metadata() -> ResponseModifier {
        ResponseModifier::StatusCode
    }
}
