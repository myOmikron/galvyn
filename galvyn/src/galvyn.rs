use std::convert::Infallible;
use std::mem;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::sync::RwLock;

use galvyn_core::registry::builder::RegistryBuilder;
use galvyn_core::router::GalvynRoute;
use galvyn_core::session;
use galvyn_core::GalvynRouter;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::core::Module;
use crate::error::GalvynError;

/// Global handle to the running galvyn server
///
/// Start creating your server by calling [`Galvyn::new`].
#[non_exhaustive]
pub struct Galvyn {
    routes: Vec<GalvynRoute>,
    shutdown: RwLock<Option<oneshot::Sender<Infallible>>>,
}

impl Galvyn {
    /// Constructs the builder to initialize and start `Galvyn`
    pub fn new() -> ModuleBuilder {
        ModuleBuilder::new()
    }

    /// Gets the global `Galvyn` instance
    ///
    /// This method should be used after [`RouterBuilder::start`] has been called.
    /// I.e. after the webserver has been started, while it is running.
    ///
    /// # Panics
    /// If galvyn has not been started yet.
    pub fn global() -> &'static Self {
        Self::try_global().unwrap_or_else(|| panic!("Galvyn has not been started yet."))
    }

    /// Gets the global `Galvyn` instance
    ///
    /// # None
    /// If galvyn has not been started yet.
    pub fn try_global() -> Option<&'static Self> {
        INSTANCE.get()
    }

    /// Quick and dirty solution to expose the registered handlers after startup
    #[doc(hidden)]
    pub fn get_routes(&self) -> &[GalvynRoute] {
        &self.routes
    }

    /// Attempts to shut down the server gracefully
    pub fn shutdown(&self) {
        let mut shutdown_tx = self
            .shutdown
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        shutdown_tx.take();
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
        let (router, routes) = mem::take(&mut self.routes).finish();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        INSTANCE.set(Galvyn {
            routes,
            shutdown: RwLock::new(Some(shutdown_tx)),
        })
            .unwrap_or_else(|_| panic!("Galvyn has already been started. There can't be more than one instance per process."));

        let socket = TcpListener::bind(socket_addr).await?;

        info!("Starting to serve webserver on http://{socket_addr}");
        let serve_future = axum::serve(socket, router.layer(session::layer()));

        #[cfg(feature = "graceful-shutdown")]
        let signal = {
            debug!("Registering signals for graceful shutdown");
            crate::graceful_shutdown::wait_for_signal()?
        };
        #[cfg(not(feature = "graceful-shutdown"))]
        let signal = std::future::pending::<()>();

        serve_future
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = signal => (),
                    _ = shutdown_rx => (),
                }
            })
            .await?;

        Ok(())
    }
}

static INSTANCE: OnceLock<Galvyn> = OnceLock::new();
