use axum::http::Method;
use axum::routing::MethodRouter;
use openapiv3::Responses;
use crate::handler_argument::HandlerArgumentFns;
use crate::internals::{SchemaGenerator};

pub trait Handler {
    /// Meta information about a [`Handler`] gathered by the [`#[handler]`](crate::handler) macro
    fn meta(&self) -> HandlerMeta;

    /// The actual function stored in an axum specific format
    fn method_router(&self) -> MethodRouter;
}

/// Meta information about a [`Handler`] gathered by the [`#[handler]`](crate::handler) macro
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

    /// The handler's return type's [`AsResponses::responses`](crate::as_responses::AsResponses::responses)
    pub responses: fn(&mut SchemaGenerator) -> Responses,

    /// The handler's arguments' [`HandlerArgument`](crate::handler_argument::HandlerArgument)'s methods
    pub handler_arguments: &'static [Option<HandlerArgumentFns>],
}
