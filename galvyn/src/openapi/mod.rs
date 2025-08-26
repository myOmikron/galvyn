//! Auto-generates an openapi document for your application

use std::any::{Any, TypeId};
use std::sync::OnceLock;

pub use openapiv3::OpenAPI;

use crate::openapi::generate::generate_openapi;
pub use crate::openapi::metadata::OpenapiMetadata;
pub use crate::openapi::router_ext::OpenapiRouterExt;

mod generate;
mod metadata;
mod router_ext;

/// Auto-generates an openapi document for your application
///
/// # Panics
/// If galvyn has not been started yet.
pub fn get_openapi() -> &'static OpenAPI {
    static OPENAPI: OnceLock<OpenAPI> = OnceLock::new();
    OPENAPI.get_or_init(|| OpenapiBuilder::default().build())
}

/// Auto-generates an openapi document for a single page
///
/// # Panics
/// If galvyn has not been started yet.
pub fn get_openapi_for_page(page: impl Any) -> OpenAPI {
    OpenapiBuilder::default().page(page).build()
}

/// Builder used to configure how to generate the openapi document
#[derive(Clone, Default)]
#[cfg_attr(doc, non_exhaustive)]
pub struct OpenapiBuilder {
    /// Should tags be omitted from the openapi document?
    pub omit_tags: bool,

    #[doc(hidden)]
    #[allow(private_interfaces)]
    pub private: OpenapiBuilderPrivate,
}
/// Private part of [`OpenapiBuilder`]
///
/// This struct exists and is private
/// 1. to mark `OpenapiBuilder` as non-exhaustive while being able to construct it with literal
/// 2. to hide the implementation details of some options behind an ergonomic api
#[derive(Clone, Default)]
struct OpenapiBuilderPrivate {
    /// `None` -> include all endpoints
    /// `Some` -> only include endpoints which are in one of the pages
    pages: Option<Vec<TypeId>>,
}
impl OpenapiBuilder {
    /// Adds a page to include in the document
    ///
    /// If no pages are specified, then every route will be included.
    pub fn page(&mut self, page: impl Any) -> &mut Self {
        assert_eq!(
            size_of_val(&page),
            0,
            "pages should be zero-sized marker types"
        );

        self.private
            .pages
            .get_or_insert_default()
            .push(page.type_id());
        self
    }

    /// Generated the openapi document
    pub fn build(&self) -> OpenAPI {
        generate_openapi(self)
    }
}
