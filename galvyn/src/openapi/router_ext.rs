//! [`GalvynRouter`] extension trait

use galvyn_core::GalvynRouter;

use crate::openapi::metadata::OpenapiMetadata;

/// Extension trait for [`GalvynRouter`]
///
/// It provides convenient methods for adding openapi related metadata
/// to a route. (For example tags)
pub trait OpenapiRouterExt {
    /// Adds a tag to all handlers in this router
    fn openapi_tag(self, tag: &'static str) -> Self;

    /// Creates a new router with a tag
    ///
    /// (Shorthand for `GalvynRouter::new().openapi_tag(...)`)
    fn with_openapi_tag(tag: &'static str) -> Self;
}

impl OpenapiRouterExt for GalvynRouter {
    fn openapi_tag(self, tag: &'static str) -> Self {
        self.metadata(OpenapiMetadata { tags: vec![tag] })
    }

    fn with_openapi_tag(tag: &'static str) -> Self {
        Self::new().openapi_tag(tag)
    }
}
