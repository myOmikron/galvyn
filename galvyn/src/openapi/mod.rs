//! Auto-generates an openapi document for your application

use std::sync::OnceLock;

pub use openapiv3::OpenAPI;

use crate::openapi::generate::generate_openapi;
pub use crate::openapi::metadata::OpenapiMetadata;
pub use crate::openapi::router_ext::OpenapiRouterExt;

mod generate;
mod metadata;
mod router_ext;

/// Auto-generates an openapi document for your application
pub fn get_openapi() -> &'static OpenAPI {
    static OPENAPI: OnceLock<OpenAPI> = OnceLock::new();
    OPENAPI.get_or_init(generate_openapi)
}
