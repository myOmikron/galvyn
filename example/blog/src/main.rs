use std::net::SocketAddr;
use std::str::FromStr;

use galvyn::contrib::TracingModule;
use galvyn::Galvyn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut galvyn = Galvyn::init();

    galvyn.register_module(TracingModule::default());

    galvyn
        .start(SocketAddr::from_str("127.0.0.1:8080")?)
        .await?;

    Ok(())
}
