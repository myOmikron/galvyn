use std::mem;
use std::net::SocketAddr;

use axum::Router;
use galvyn_core::re_exports::rorm::Database;
use galvyn_core::registry::builder::RegistryBuilder;
use galvyn_core::session;
use galvyn_core::GalvynRouter;
use tokio::net::TcpListener;
use tracing::info;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::core::Module;
use crate::error::GalvynError;

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
        let registry = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(Level::INFO.as_str())))
            .with(tracing_subscriber::fmt::layer());

        registry.init();

        let mut this = ModuleBuilder::default();
        this.register_module::<Database>();
        this
    }

    /// Register a module
    pub fn register_module<T: Module>(&mut self) -> &mut Self {
        self.modules.register_module::<T>();
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
        let router = Router::from(mem::take(&mut self.routes)).layer(session::layer());

        let socket = TcpListener::bind(socket_addr).await?;

        info!("Starting to serve webserver on http://{socket_addr}");
        axum::serve(socket, router).await?;

        Ok(())
    }
}
