use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tracing::debug;
use tracing::info;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::core::Module;
use crate::error::GalvynError;

pub struct Galvyn {
    modules: Vec<Box<dyn Module>>,
}

impl Galvyn {
    pub fn init() -> Self {
        let registry = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(Level::INFO.as_str())))
            .with(tracing_subscriber::fmt::layer());

        registry.init();

        Self { modules: vec![] }
    }

    /// Register a module
    pub fn register_module(&mut self, module: impl Module + 'static) {
        module.init_stage();

        debug!("Register module {}", module.name());

        self.modules.push(Box::new(module));
    }

    /// Initializes all modules and start the webserver
    pub async fn start(&self, socket_addr: SocketAddr) -> Result<(), GalvynError> {
        // Run router stage
        let routes = self.router_stage();

        let socket = TcpListener::bind(socket_addr).await?;

        info!("Starting to serve webserver on http://{socket_addr}");
        axum::serve(socket, routes).await?;

        Ok(())
    }
}

// Private methods
impl Galvyn {
    fn init_stage(&self) {
        for m in &self.modules {
            m.init_stage();
        }
    }

    fn router_stage(&self) -> Router {
        let mut root = Router::new();

        for m in &self.modules {
            root = m.router_stage(root);
        }

        root
    }
}
