use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::Route;
use axum::routing::Router;
use std::convert::Infallible;
use std::ops::Deref;
use tower::Layer;
use tower::Service;

use crate::handler::{GalvynHandler, HandlerMeta};
// use crate::{SwaggapiPage, PAGE_OF_EVERYTHING};

/// An `GalvynRouter` combines several [`SwaggapiHandler`] under a common path.
///
/// It is also responsible for adding them to [`SwaggapiPage`]s once mounted to your application.
///
/// TODO: update these docs
#[derive(Debug, Default)]
pub struct GalvynRouter {
    /// The contained handlers
    handlers: Vec<MutHandlerMeta>,

    /// The underlying axum router
    router: Router,

    /* Parameters added to new handlers */
    /// A base path all handlers are routed under
    ///
    /// This is effectively remembers the argument actix' `Scope` was created with.
    /// Since `Router` doesn't take a path, this will always be empty for axum.
    path: String,
    //
    // /// Changes have to be applied to already existing `handlers` manually
    // pages: Vec<&'static SwaggapiPage>,
    //
    // /// Changes have to be applied to already existing `handlers` manually
    // tags: Vec<&'static str>,
}

impl GalvynRouter {
    /// Create a new router
    ///
    /// It wraps an axum [`Router`] internally and should be added to your application's router using [`Router::merge`]:
    /// ```rust
    /// # use axum::Router;
    /// # use swaggapi::GalvynRouter;
    /// let app = Router::new().merge(GalvynRouter::new("/api"));
    /// ```
    ///
    /// TODO: update these docs
    pub fn new() -> Self {
        Self::default()
    }

    // /// Create a new router with a tag
    // ///
    // /// (Shorthand for `GalvynRouter::new().tag(...)`)
    // pub fn with_tag(tag: &'static str) -> Self {
    //     Self::new().tag(tag)
    // }

    /// Add a handler to the router
    pub fn handler(mut self, handler: impl GalvynHandler) -> Self {
        self.push_handler(MutHandlerMeta::new(handler.meta()));
        self.router = self
            .router
            .route(&handler.meta().path, handler.method_router());
        self
    }

    // /// Attach a [`SwaggapiPage`] this router's handlers will be added to
    // pub fn page(mut self, page: &'static SwaggapiPage) -> Self {
    //     self.pages.push(page);
    //     for handler in &mut self.handlers {
    //         handler.pages.insert(page);
    //     }
    //     self
    // }
    //
    // /// Add a tag to all of this router's handlers
    // pub fn tag(mut self, tag: &'static str) -> Self {
    //     self.tags.push(tag);
    //     for handler in &mut self.handlers {
    //         handler.tags.insert(tag);
    //     }
    //     self
    // }

    /// Adds a [`MutHandlerMeta`] after adding this router's `path`, `tags` and `pages` to it
    fn push_handler(&mut self, mut handler: MutHandlerMeta) {
        if !self.path.is_empty() {
            handler.path = format!("{}{}", self.path, handler.path);
        }
        // handler.tags.extend(self.tags.iter().copied());
        // handler.pages.extend(self.pages.iter().copied());
        self.handlers.push(handler);
    }

    /// Adds the handlers to their api pages and returns the contained framework impl
    fn finish(self) -> Router {
        // for mut handler in self.handlers {
        //     PAGE_OF_EVERYTHING.add_handler(&handler);
        //     for page in handler.pages.iter() {
        //         page.add_handler(&handler);
        //     }
        // }
        return self.router;
    }

    /// Calls [`Router::nest`] while preserving api information
    pub fn nest(mut self, path: &str, other: GalvynRouter) -> Self {
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

impl From<GalvynRouter> for Router {
    fn from(router: GalvynRouter) -> Self {
        router.finish()
    }
}

/// A wrapped [`HandlerMeta`] used inside [`GalvynRouter`] to allow modifications.
#[derive(Debug)]
pub(crate) struct MutHandlerMeta {
    /// The original unmodified [`HandlerMeta`]
    pub original: HandlerMeta,

    /// The handler's modified path
    pub path: String,
}
impl MutHandlerMeta {
    /// Constructs a new `MutHandlerMeta`
    pub fn new(original: HandlerMeta) -> Self {
        Self {
            path: original.path.to_string(),
            // tags: PtrSet::from_iter(original.tags.iter().copied()),
            // pages: PtrSet::new(),
            original,
        }
    }
}
impl Deref for MutHandlerMeta {
    type Target = HandlerMeta;

    fn deref(&self) -> &Self::Target {
        &self.original
    }
}
