pub mod re_exports {
    pub use tracing;
}

use galvyn_core::re_exports::axum;
use galvyn_core::Module;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::DefaultOnResponse;
use tower_http::trace::TraceLayer;
use tracing::Level;

pub struct TracingModule {
    pub default_level: Level,
}

impl Default for TracingModule {
    fn default() -> Self {
        Self {
            default_level: Level::INFO,
        }
    }
}

impl Module for TracingModule {
    fn name(&self) -> &str {
        "galvyn::contrib::tracing"
    }

    fn router_stage(&self, root: axum::routing::Router) -> axum::routing::Router {
        root.layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                // Disable automatic failure logger because any handler returning a 500 should have already logged its reason™
                .on_failure(()),
        )
    }
}
