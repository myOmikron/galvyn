//! Parts of the http api for the frontend
//!
//! This included the router as well as the handlers and schemas

use axum::Extension;
use axum::Router;
use openidconnect::core::CoreClient;
use swaggapi::ApiContext;
use swaggapi::SwaggapiPageBuilder;
use tower::ServiceBuilder;

use crate::http::middlewares::auth_required::auth_required;
use crate::http::middlewares::role_required::RoleRequiredLayer;
use crate::models::UserRole;

pub mod auth;
pub mod oidc;
pub mod user_invites;
pub mod users;
pub mod ws;

/// The swagger page for the frontend
pub static FRONTEND_API_V1: SwaggapiPageBuilder = SwaggapiPageBuilder::new()
    .title("Frontend")
    .filename("frontend.json");

/// Create the router for the Frontend API
pub fn initialize(oidc_client: Option<CoreClient>) -> ApiContext<Router> {
    let mut oidc_context = ApiContext::new()
        .tag("OpenId Connect")
        .handler(oidc::handler_common::oidc_login)
        .handler(oidc::handler_common::finish_login);

    if let Some(oidc_client) = oidc_client {
        oidc_context =
            oidc_context.route_layer(ServiceBuilder::new().layer(Extension(oidc_client)));
    };

    ApiContext::new().nest(
        "/v1",
        ApiContext::new()
            .nest(
                "/common",
                ApiContext::new()
                    .nest("/oidc", oidc_context)
                    .nest(
                        "/auth",
                        ApiContext::new()
                            .tag("Auth")
                            .handler(auth::handler_common::get_login_flows)
                            .handler(auth::handler_common::login_webauthn)
                            .handler(auth::handler_common::login_password)
                            .route_layer(ServiceBuilder::new().concurrency_limit(10))
                            .handler(auth::handler_common::verify_webauthn)
                            .handler(auth::handler_common::verify_totp)
                            .handler(auth::handler_common::complete_auth_webauthn)
                            .handler(auth::handler_common::logout),
                    )
                    .merge(
                        ApiContext::new()
                            .nest(
                                "/users",
                                ApiContext::new()
                                    .tag("users")
                                    .handler(users::handler_common::get_me)
                                    .handler(users::handler_common::change_password),
                            )
                            .layer(
                                ServiceBuilder::new()
                                    .layer(axum::middleware::from_fn(auth_required)),
                            ),
                    ),
            )
            .nest(
                "/admin",
                ApiContext::new()
                    .nest(
                        "/users",
                        ApiContext::new()
                            .tag("Users")
                            .handler(users::handler_admin::get_all_users)
                            .handler(users::handler_admin::set_user_permissions)
                            .handler(users::handler_admin::delete_user),
                    )
                    .nest(
                        "/user-invites",
                        ApiContext::new()
                            .tag("User Invites")
                            .handler(user_invites::handler_common::get_user_invite)
                            .handler(user_invites::handler_common::accept_with_password)
                            .handler(user_invites::handler_common::accept_with_webauthn)
                            .handler(user_invites::handler_common::complete_invites_webauthn),
                    )
                    .layer(
                        ServiceBuilder::new()
                            .layer(RoleRequiredLayer::new(UserRole::Administrator)),
                    ),
            ),
    )
}
