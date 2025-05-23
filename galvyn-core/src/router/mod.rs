use std::convert::Infallible;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::Route;
use axum::routing::Router;
use tower::Layer;
use tower::Service;

use crate::handler::GalvynHandler;
use crate::handler::HandlerMeta;

pub use self::extension::{RouteExtension, RouteExtensions};

mod extension;

/// An `GalvynRouter` combines several [`SwaggapiHandler`] under a common path.
///
/// It is also responsible for adding them to [`SwaggapiPage`]s once mounted to your application.
///
/// TODO: update these docs
#[derive(Debug, Default)]
pub struct GalvynRouter {
    /// The contained handlers
    handlers: Vec<GalvynRoute>,

    /// The underlying axum router
    router: Router,

    /// Route extensions implicitly added to all routes added to this router
    extensions: RouteExtensions,
}

impl GalvynRouter {
    /// Creates a new router
    ///
    /// TODO: update these docs
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new router with an extension
    ///
    /// (Shorthand for `GalvynRouter::new().extension(...)`)
    pub fn with_extension(extension: impl RouteExtension) -> Self {
        Self::new().extension(extension)
    }

    /// Adds a handler to the router
    pub fn handler(mut self, handler: impl GalvynHandler) -> Self {
        self.push_handler(GalvynRoute::new(handler.meta()));
        self.router = self
            .router
            .route(&handler.meta().path, handler.method_router());
        self
    }

    /// Adds a `RouteExtension` to every handler added to this router.
    ///
    /// The extension will be added to all handlers,
    /// regardless of whether the handler was added before or after this method was called.
    ///
    /// If an extension of this type has already been added then the two extensions will be merged.
    pub fn extension(mut self, extension: impl RouteExtension) -> Self {
        for handler in &mut self.handlers {
            handler.extensions.insert(extension.clone());
        }
        self.extensions.insert(extension);
        self
    }

    /// Adds a [`GalvynRoute`] after adding this router's `path`, `tags` and `pages` to it
    fn push_handler(&mut self, mut handler: GalvynRoute) {
        handler.extensions.merge(&self.extensions);
        self.handlers.push(handler);
    }

    pub fn finish(self) -> (Router, Vec<GalvynRoute>) {
        (self.router, self.handlers)
    }

    /// Calls [`Router::nest`] while preserving api information
    #[track_caller]
    pub fn nest(mut self, path: &str, other: GalvynRouter) -> Self {
        if path.is_empty() || path == "/" {
            panic!("Nesting at the root is no longer supported. Use merge instead.");
        }
        if !path.starts_with('/') {
            panic!("Paths must start with a slash.");
        }

        for mut handler in other.handlers {
            // Code taken from `path_for_nested_route` in `axum/src/routing/path_router.rs`
            handler.path = if path.ends_with('/') {
                format!("{path}{}", handler.path.trim_start_matches('/'))
            } else if handler.path == "/" {
                path.into()
            } else {
                format!("{path}{}", handler.path)
            };

            self.push_handler(handler);
        }

        self.router = self.router.nest(path, other.router);
        self
    }

    /// Calls [`Router::merge`] while preserving api information
    pub fn merge(mut self, other: GalvynRouter) -> Self {
        for handler in other.handlers {
            self.push_handler(handler);
        }
        self.router = self.router.merge(other.router);
        self
    }

    /// Apply a [`tower::Layer`] to all routes in the router.
    ///
    /// See [`Router::layer`] for more details.
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        self.router = self.router.layer(layer);
        self
    }

    /// Apply a [`tower::Layer`] to the router that will only run if the request matches a route.
    ///
    /// See [`Router::route_layer`] for more details.
    pub fn route_layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        self.router = self.router.route_layer(layer);
        self
    }
}

/// A route associates a url and method with a handler
///
/// It also stores extensions which can be used for reflection.
#[derive(Debug)]
pub struct GalvynRoute {
    /// Meta information about the route's handler
    ///
    /// This includes the route's method
    pub handler: HandlerMeta,

    /// The route's path i.e. url without the host information
    pub path: String,

    /// Arbitrary additional meta information associated with the route
    ///
    /// For example openapi tags.
    pub extensions: RouteExtensions,
}
impl GalvynRoute {
    /// Constructs a new `GalvynRoute`
    pub fn new(original: HandlerMeta) -> Self {
        Self {
            path: original.path.to_string(),
            // tags: PtrSet::from_iter(original.tags.iter().copied()),
            // pages: PtrSet::new(),
            handler: original,
            extensions: RouteExtensions::default(),
        }
    }
}
