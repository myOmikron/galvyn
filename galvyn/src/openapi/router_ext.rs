//! [`GalvynRouter`] extension trait

use galvyn_core::GalvynRouter;
use std::any::Any;

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

    /// Associates a page with all handlers in this router
    ///
    /// ```
    /// # use galvyn::openapi::OpenapiRouterExt;
    /// # use galvyn_core::GalvynRouter;
    /// struct FrontendApi;
    /// struct MonitoringApi;
    ///
    /// GalvynRouter::new()
    ///     .nest(
    ///         "/api/frontend",
    ///         GalvynRouter::new()
    ///             .openapi_page(FrontendApi),
    ///     )
    ///     .nest(
    ///         "/api/monitoring",
    ///         GalvynRouter::new()
    ///             .openapi_page(MonitoringApi),
    ///     );
    /// ```
    fn openapi_page(self, page: impl Any) -> Self;

    /// Creates a new router with a page
    ///
    /// (Shorthand for `GalvynRouter::new().openapi_page(...)`)
    ///
    /// ```
    /// # use galvyn::openapi::OpenapiRouterExt;
    /// # use galvyn_core::GalvynRouter;
    /// struct FrontendApi;
    /// struct MonitoringApi;
    ///
    /// GalvynRouter::new()
    ///     .nest(
    ///         "/api/frontend",
    ///         GalvynRouter::with_openapi_page(FrontendApi),
    ///     )
    ///     .nest(
    ///         "/api/monitoring",
    ///         GalvynRouter::with_openapi_page(MonitoringApi),
    ///     );
    /// ```
    fn with_openapi_page(page: impl Any) -> Self;
}

impl OpenapiRouterExt for GalvynRouter {
    fn openapi_tag(self, tag: &'static str) -> Self {
        self.metadata(OpenapiMetadata {
            tags: vec![tag],
            pages: Vec::new(),
        })
    }

    fn with_openapi_tag(tag: &'static str) -> Self {
        Self::new().openapi_tag(tag)
    }

    fn openapi_page(self, page: impl Any) -> Self {
        assert_eq!(
            size_of_val(&page),
            0,
            "pages should be zero-sized marker types"
        );

        self.metadata(OpenapiMetadata {
            tags: Vec::new(),
            pages: vec![page.type_id()],
        })
    }

    fn with_openapi_page(page: impl Any) -> Self {
        Self::new().openapi_page(page)
    }
}
