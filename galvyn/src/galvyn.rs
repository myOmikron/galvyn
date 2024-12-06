use std::mem;
use std::net::SocketAddr;

use crate::core::Module;
use crate::error::GalvynError;
use axum::Router;
use galvyn_core::registry::builder::RegistryBuilder;
use galvyn_core::{session, GalvynRouter};
use rorm::Database;
use tokio::net::TcpListener;
use tracing::info;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Default)]
pub struct Galvyn {
    modules: RegistryBuilder,
    routes: GalvynRouter,
}

impl Galvyn {
    pub fn init() -> Self {
        let registry = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(Level::INFO.as_str())))
            .with(tracing_subscriber::fmt::layer());

        registry.init();

        let mut galvyn = Galvyn::default();
        galvyn.register_module::<Database>();
        galvyn
    }

    pub fn add_routes(&mut self, routes: GalvynRouter) -> &mut Self {
        self.routes = mem::take(&mut self.routes).merge(routes);
        self
    }

    /// Register a module
    pub fn register_module<T: Module>(&mut self) -> &mut Self {
        self.modules.register_module::<T>();
        self
    }

    /// Initializes all modules and start the webserver
    pub async fn start(&mut self, socket_addr: SocketAddr) -> Result<(), GalvynError> {
        self.modules.init().await?;

        let router = Router::from(mem::take(&mut self.routes)).layer(session::layer());

        let socket = TcpListener::bind(socket_addr).await?;

        info!("Starting to serve webserver on http://{socket_addr}");
        axum::serve(socket, router).await?;

        Ok(())
    }
}
