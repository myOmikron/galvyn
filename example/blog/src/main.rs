mod auth_models;

use std::any::type_name;
use std::net::SocketAddr;
use std::str::FromStr;

use galvyn::contrib::tracing::TracingModule;
use galvyn::{get, Galvyn};

#[get("/index")]
async fn test<const N: usize, T: 'static>() -> String {
    format!("<{N}, {}>", type_name::<T>())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut galvyn = Galvyn::init();

    galvyn.register_module(TracingModule::default());

    galvyn
        .start(SocketAddr::from_str("127.0.0.1:8080")?)
        .await?;

    Ok(())
}
