use crate::core::Module;
use crate::error::GalvynError;
use galvyn_core::registry::builder::RegistryBuilder;
use galvyn_core::router::GalvynRoute;
use galvyn_core::session;
use galvyn_core::GalvynRouter;
use std::mem;
use std::net::SocketAddr;
use std::sync::RwLock;
use tokio::net::TcpListener;
use tracing::Level;
use tracing::{debug, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[non_exhaustive]
pub struct Galvyn;

impl Galvyn {
    pub fn new() -> ModuleBuilder {
        ModuleBuilder::new()
    }
}

#[derive(Default)]
pub struct ModuleBuilder {
    modules: RegistryBuilder,
}

impl ModuleBuilder {
    fn new() -> ModuleBuilder {
        #[cfg(feature = "panic-hook")]
        crate::panic_hook::set_panic_hook();

        let registry = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(Level::INFO.as_str())))
            .with(tracing_subscriber::fmt::layer());

        if registry.try_init().is_ok() {
            debug!("Initialized galvyn's subscriber");
        } else {
            debug!("Using external subscriber");
        }

        ModuleBuilder::default()
    }

    /// Register a module
    pub fn register_module<T: Module>(&mut self, setup: T::Setup) -> &mut Self {
        self.modules.register_module::<T>(setup);
        self
    }

    pub async fn init_modules(&mut self) -> Result<RouterBuilder, GalvynError> {
        self.modules.init().await?;
        Ok(RouterBuilder {
            routes: GalvynRouter::new(),
        })
    }
}

pub struct RouterBuilder {
    routes: GalvynRouter,
}

impl RouterBuilder {
    /// Adds a router to the builder
    pub fn add_routes(&mut self, router: GalvynRouter) -> &mut Self {
        let this = mem::take(&mut self.routes);
        self.routes = this.merge(router);
        self
    }

    /// Starts the webserver
    pub async fn start(&mut self, socket_addr: SocketAddr) -> Result<(), GalvynError> {
        let (router, handlers) = mem::take(&mut self.routes).finish();

        *HANDLERS.write().unwrap() = handlers.leak();

        let socket = TcpListener::bind(socket_addr).await?;

        info!("Starting to serve webserver on http://{socket_addr}");
        let serve_future = axum::serve(socket, router.layer(session::layer()));

        debug!("Registering signals for graceful shutdown");
        #[cfg(feature = "graceful-shutdown")]
        let serve_future =
            serve_future.with_graceful_shutdown(crate::graceful_shutdown::wait_for_signal()?);

        serve_future.await?;

        Ok(())
    }
}

static HANDLERS: RwLock<&'static [GalvynRoute]> = RwLock::new(&[]);

/// Quick and dirty solution to expose the registered handlers after startup
#[doc(hidden)]
pub fn get_routes() -> &'static [GalvynRoute] {
    *HANDLERS.read().unwrap()
}
