//! Auto-generates an openapi document for your application

use std::any::Any;
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
    OPENAPI.get_or_init(|| generate_openapi(None))
}

/// Auto-generates an openapi document a single page
///
/// # Panics
/// If galvyn has not been started yet.
pub fn get_openapi_for_page(page: impl Any) -> OpenAPI {
    assert_eq!(
        size_of_val(&page),
        0,
        "pages should be zero-sized marker types"
    );

    generate_openapi(Some(page.type_id()))
}
