#![warn(missing_docs)]
#![warn(clippy::todo)]

pub mod as_responses;
mod context;
pub mod handler_argument;
pub mod internals;
mod macro_docs;
mod page;
pub mod utils;

pub use macro_docs::*;

pub use self::context::ApiContext;
pub use self::page::PageOfEverything;
pub use self::page::SwaggapiPage;
pub use self::page::SwaggapiPageBuilder;

/// Reexports for macros and implementors
pub mod re_exports {
    pub use axum;
    pub use indexmap;
    pub use mime;
    pub use openapiv3;
    pub use schemars;
}
