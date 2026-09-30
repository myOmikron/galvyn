use axum::http::Method;

#[cfg(doc)]
use crate::handler::request_body::RequestBody;
#[cfg(doc)]
use crate::handler::request_part::RequestPart;
#[cfg(doc)]
use crate::handler::response_body::ResponseBody;
#[cfg(doc)]
use crate::handler::response_part::ResponsePart;
use crate::schema_generator::SchemaGenerator;

/// Context used by [`RequestPart`], [`RequestBody`], [`ResponsePart`] and [`ResponseBody`].
///
/// Most noteworthy, it contains the [`SchemaGenerator`] implementors can use to generate json schemas.
///
/// It also wraps some additional context about the endpoint for which the schemas should be generated.
#[non_exhaustive]
pub struct EndpointContext<'ctx> {
    /// State for generating schemas from types implementing [`JsonSchema`](schemars::JsonSchema)
    pub generator: &'ctx mut SchemaGenerator,

    /// HTTP method of the endpoint to generate schemas for
    pub method: &'ctx Method,

    /// Url path of the endpoint to generate schemas for
    pub path: &'ctx str,
}

impl<'ctx> EndpointContext<'ctx> {
    #[doc(hidden)]
    pub fn _new(
        generator: &'ctx mut SchemaGenerator,
        method: &'ctx Method,
        path: &'ctx str,
    ) -> Self {
        Self {
            generator,
            method,
            path,
        }
    }
}
