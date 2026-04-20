use std::mem;
use std::net::SocketAddr;

use galvyn_core::modules::shutdown::Shutdown;
use galvyn_core::modules::shutdown::ShutdownSetup;
use galvyn_core::registry::builder::RegistryBuilder;
use galvyn_core::router::GalvynRoute;
use galvyn_core::GalvynRouter;
use tokio::net::TcpListener;
use tokio::sync::SetOnce;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::core::Module;
use crate::error::GalvynError;
use crate::panic_hook::set_panic_hook;

/// Global handle to the running galvyn server
///
/// Start creating your server by calling [`Galvyn::new`].
#[non_exhaustive]
pub struct Galvyn {
    routes: Vec<GalvynRoute>,
}

/// General setup options for galvyn.
///
/// Most modules will provide their own setup options.
#[derive(Default)]
#[cfg_attr(doc, non_exhaustive)]
pub struct GalvynSetup {
    /// Disables galvyn's session layer
    ///
    /// If you want to bring your own.
    #[cfg(feature = "sessions")]
    pub disable_sessions: bool,

    /// Disables galvyn's [panic hook](crate::panic_hook).
    ///
    /// Galvyn's panic hook applies globally (including non-galvyn code)
    /// and emits `error!` events instead of printing to stderr when a panic is raised.
    /// (Whether the panic is caught or not does not matter.)
    pub disable_panic_hook: bool,

    /// Setup how the server should behave on shutdown
    pub shutdown: ShutdownSetup,

    #[doc(hidden)]
    pub _non_exhaustive: (),
}

impl Galvyn {
    /// Constructs the builder to initialize and start `Galvyn`
    pub fn builder(setup: GalvynSetup) -> ModuleBuilder {
        ModuleBuilder::new(setup)
    }

    /// Gets the global `Galvyn` instance
    ///
    /// This method should be used after [`RouterBuilder::start`] has been called.
    /// I.e. after the webserver has been started, while it is running.
    ///
    /// # Panics
    /// If galvyn has not been started yet.
    ///
    /// If you want to wait until it has started, use [`Galvyn::started`].
    pub fn global() -> &'static Self {
        Self::try_global().unwrap_or_else(|| panic!("Galvyn has not been started yet."))
    }

    /// Gets the global `Galvyn` instance
    ///
    /// # None
    /// If galvyn has not been started yet.
    ///
    /// If you want to wait until it has started, use [`Galvyn::started`].
    pub fn try_global() -> Option<&'static Self> {
        INSTANCE.get()
    }

    /// Waits for `Galvyn` to start and returns its global instance
    ///
    /// If you can't use `async` and know the server has started, use [`Galvyn::global`] or [`Galvyn::try_global`].
    pub async fn global_wait() -> &'static Self {
        INSTANCE.wait().await
    }

    /// Quick and dirty solution to expose the registered handlers after startup
    #[doc(hidden)]
    pub fn get_routes(&self) -> &[GalvynRoute] {
        &self.routes
    }

    /// Attempts to shut down the server gracefully
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn shutdown(&self) {
        Shutdown::global().start();
    }

    /// Waits until the server started a graceful shutdown
    ///
    /// This method can be used by tasks to terminate themselves.
    pub async fn shutdown_started(&self) {
        Shutdown::global().wait_for_started().await;
    }

    /// Constructs a guard which has to be dropped before a graceful shutdown can complete
    ///
    /// This has no effect on a forced shutdown.
    pub fn block_shutdown(&self) -> impl Drop + Send + Sync + 'static {
        Shutdown::global().block()
    }

    /// Force the server to stop
    ///
    /// This will cause the [`start`](RouterBuilder::start) method to return immediately.
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn kill(&self) {
        Shutdown::global().force_done();
    }
}

/// Intermediate build for your [`Galvyn`] instance
///
/// Register all [`Module`]s you require using [`register_module`](Self::register_module)
/// and then call finish this builder step using [`init_modules`](Self::init_modules).
///
/// The next step will be adding http routes. (See [`RouterBuilder`])
#[derive(Default)]
pub struct ModuleBuilder {
    modules: RegistryBuilder,
    setup: GalvynSetup,
}

impl ModuleBuilder {
    fn new(mut setup: GalvynSetup) -> ModuleBuilder {
        if !setup.disable_panic_hook {
            set_panic_hook();
        }

        let registry = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(Level::INFO.as_str())))
            .with(tracing_subscriber::fmt::layer());

        if registry.try_init().is_ok() {
            debug!("Initialized galvyn's subscriber");
        } else {
            debug!("Using external subscriber");
        }

        let mut modules = RegistryBuilder::new();
        modules.register_module::<Shutdown>(mem::take(&mut setup.shutdown));
        ModuleBuilder { modules, setup }
    }

    /// Register a module
    pub fn register_module<T: Module>(&mut self, setup: T::Setup) -> &mut Self {
        self.modules.register_module::<T>(setup);
        self
    }

    /// Initializes all modules and returns a builder for the routes
    ///
    /// This method takes `&mut self` for convenience.
    /// The `ModuleBuilder` should not be used anymore after calling this method.
    // (It won't cause any errors or panics, `self` will simply be "empty")
    pub async fn init_modules(&mut self) -> Result<RouterBuilder, GalvynError> {
        self.modules.init().await?;
        Ok(RouterBuilder {
            routes: GalvynRouter::new(),
            setup: mem::take(&mut self.setup),
        })
    }
}

pub struct RouterBuilder {
    routes: GalvynRouter,
    setup: GalvynSetup,
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
        #[allow(unused_mut, reason = "Usage is behind feature flags")]
        let (mut router, routes) = mem::take(&mut self.routes).finish();

        #[cfg(feature = "sessions")]
        if !self.setup.disable_sessions {
            router = router.layer(galvyn_core::session::layer());
        }

        INSTANCE.set(Galvyn {
            routes,
        })
            .unwrap_or_else(|_| panic!("Galvyn has already been started. There can't be more than one instance per process."));
        let shutdown = Shutdown::global();

        #[cfg(feature = "graceful-shutdown")]
        {
            debug!("Registering signals for graceful shutdown");
            let signal = crate::graceful_shutdown::wait_for_signal()?;
            tokio::spawn(async move {
                signal.await;
                shutdown.start();
            });
        }

        info!("Starting to serve webserver on http://{socket_addr}");
        let socket = TcpListener::bind(socket_addr).await?;
        let serve = axum::serve(socket, router).with_graceful_shutdown(shutdown.wait_for_started());
        {
            let _blocker = shutdown.block();
            if serve.await.is_err() {
                // Axum said they would never error.
                // They would only return (with ok) when the graceful shutdown finished.
                error!("Unreachable, this is a bug in galvyn");
            }
        }

        shutdown.wait_for_done().await;
        Ok(())
    }
}

static INSTANCE: SetOnce<Galvyn> = SetOnce::const_new();
