use std::any::type_name;
use std::net::SocketAddr;
use std::str::FromStr;

use galvyn::contrib::auth::AuthModule;
use galvyn::contrib::settings::ApplicationSettings;
use galvyn::contrib::settings::ApplicationSettingsExt;
use galvyn::contrib::settings::SettingsStore;
use galvyn::core::re_exports::axum::response::IntoResponse;
use galvyn::core::re_exports::axum::response::Response;
use galvyn::core::re_exports::axum::Json;
use galvyn::core::stuff::api_error::ApiError;
use galvyn::core::stuff::api_error::ApiResult;
use galvyn::core::stuff::api_json::ApiJson;
use galvyn::core::GalvynRouter;
use galvyn::core::Module;
use galvyn::get;
use galvyn::openapi::OpenapiRouterExt;
use galvyn::post;
use galvyn::rorm::Database;
use galvyn::Galvyn;

use crate::settings::Settings;

mod settings;

#[get("/index")]
async fn test<const N: usize, T: 'static>() -> String {
    format!("<{N}, {}>", type_name::<T>())
}

#[get("/openapi")]
async fn openapi() -> Response {
    Json(galvyn::openapi::get_openapi()).into_response()
}

#[get("/origin")]
async fn get_origin() -> String {
    Settings::global().origin.get()
}

#[post("/origin")]
async fn set_origin(ApiJson(new_origin): ApiJson<String>) -> ApiResult<()> {
    Settings::global()
        .origin
        .set(new_origin)
        .await
        .map_err(ApiError::map_server_error("Failed to set origin"))
}

#[post("/shutdown")]
async fn shutdown() {
    Galvyn::global().shutdown();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Galvyn::builder(Default::default())
        .register_module::<Database>(Default::default())
        .register_module::<SettingsStore>(Default::default())
        .register_module::<<Settings as ApplicationSettingsExt>::Module>(Default::default())
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
                .handler(shutdown)
                .handler(openapi),
        )
        .start(SocketAddr::from_str("127.0.0.1:8080")?)
        .await?;

    Ok(())
}
