use crate::handler_argument::HandlerArgumentFns;
use crate::internals::SchemaGenerator;
use axum::http::Method;
use axum::routing::MethodRouter;
use openapiv3::Responses;

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

    /// The handler's return type's [`AsResponses::responses`](crate::as_responses::AsResponses::responses)
    pub responses: fn(&mut SchemaGenerator) -> Responses,

    /// The handler's arguments' [`HandlerArgument`](crate::handler_argument::HandlerArgument)'s methods
    pub handler_arguments: &'static [Option<HandlerArgumentFns>],
}
