use axum::http::Method;

use crate::schema_generator::SchemaGenerator;

/// Context used by [`RequestPart`], [`RequestBody`], [`ResponsePart`] and [`ResponseBody`].
///
/// Most noteworthy, it contains the [`SchemaGenerator`] implementors can use to generate json schemas.
///
/// It also wraps some additional context about the endpoint for which the schemas should be generated.
#[non_exhaustive]
pub struct EndpointContext<'ctx> {
    /// State for generating schemas from types implementing [`JsonSchema`]
    pub generator: &'ctx mut SchemaGenerator,
}

impl<'ctx> EndpointContext<'ctx> {
    #[doc(hidden)]
    pub fn _new(generator: &'ctx mut SchemaGenerator) -> Self {
        Self { generator }
    }
}
