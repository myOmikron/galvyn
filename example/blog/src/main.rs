use std::any::type_name;
use std::net::SocketAddr;
use std::str::FromStr;

use galvyn::contrib::auth::AuthModule;
use galvyn::rorm::Database;
use galvyn::{get, Galvyn};

use galvyn::core::re_exports::axum::response::{IntoResponse, Response};
use galvyn::core::re_exports::axum::Json;
use galvyn::core::{GalvynRouter, Module};
use galvyn::openapi::OpenapiRouterExt;

#[get("/index")]
async fn test<const N: usize, T: 'static>() -> String {
    format!("<{N}, {}>", type_name::<T>())
}

#[get("/openapi")]
async fn openapi() -> Response {
    Json(galvyn::openapi::get_openapi()).into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Galvyn::new()
        .register_module::<Database>(Default::default())
        .register_module::<AuthModule>(Default::default())
        .init_modules()
        .await?
        .add_routes(
            GalvynRouter::with_openapi_tag("Auth Module")
                .nest("/auth", AuthModule::global().handler.as_router()),
        )
        .add_routes(
            GalvynRouter::new()
                .openapi_tag("Main")
                .handler(test::<1337, ()>)
                .handler(openapi),
        )
        .start(SocketAddr::from_str("127.0.0.1:8080")?)
        .await?;

    Ok(())
}
