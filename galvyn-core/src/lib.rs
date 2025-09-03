pub use self::handler::GalvynHandler;
pub use self::router::GalvynRouter;
pub use self::schemaless_json::SchemalessJson;
pub use crate::module::*;

pub mod re_exports {
    pub use axum;
    pub use mime;
    pub use rorm;
    pub use schemars;
    pub use time;
    pub use uuid;

    #[cfg(feature = "opentelemetry")]
    pub use opentelemetry;
    #[cfg(feature = "opentelemetry")]
    pub use opentelemetry_otlp;
    #[cfg(feature = "opentelemetry")]
    pub use opentelemetry_sdk;
    #[cfg(feature = "opentelemetry")]
    pub use tracing_opentelemetry;
}

pub mod handler;
#[doc(hidden)]
pub mod macro_utils;
pub mod middleware;
pub mod module;
#[doc(hidden)]
pub mod router;
pub mod schema_generator;
mod schemaless_json;
pub mod session;
pub mod stuff;
mod util_macros;

pub use self::module::Module;
