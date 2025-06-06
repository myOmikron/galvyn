#[cfg(feature = "contrib")]
pub mod contrib {
    pub use galvyn_contrib_auth as auth;
    // pub use galvyn_contrib_tracing as tracing;
}

/// Re-export of [`rorm`](galvyn_core::re_exports::rorm)
pub mod rorm {
    pub use galvyn_core::re_exports::rorm::*;
    /// Re-export from [`rorm`](galvyn_core::re_exports::rorm::DbEnum)
    pub use galvyn_macros::DbEnum;
    /// Re-export from [`rorm`](galvyn_core::re_exports::rorm::Model)
    pub use galvyn_macros::Model;
    /// Re-export from [`rorm`](galvyn_core::re_exports::rorm::Patch)
    pub use galvyn_macros::Patch;
}

pub mod core {
    pub use galvyn_core::*;
}

pub use crate::galvyn::*;

pub mod error;
mod galvyn;
#[cfg(feature = "graceful-shutdown")]
mod graceful_shutdown;
mod macro_docs;
#[cfg(feature = "openapi")]
pub mod openapi;
#[cfg(feature = "panic-hook")]
pub mod panic_hook;

pub use macro_docs::*;
