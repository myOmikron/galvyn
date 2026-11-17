pub mod thread;
pub mod user;

use galvyn::core::re_exports::axum::extract::FromRequestParts;
use galvyn::core::re_exports::axum::http::request::Parts;
use galvyn::core::session::Session;
use galvyn::core::stuff::api_error::ApiError;
use galvyn::core::GalvynRouter;
use galvyn::core::Module;
use galvyn::rorm;
use galvyn::rorm::Database;

use crate::models::user::User;

pub fn get_router() -> GalvynRouter {
    GalvynRouter::new().nest(
        "/api",
        GalvynRouter::new()
            .nest(
                "/user",
                GalvynRouter::new()
                    .handler(user::register)
                    .handler(user::login)
                    .handler(user::logout)
                    .handler(user::delete)
                    .handler(user::profile),
            )
            .nest(
                "/thread",
                GalvynRouter::new()
                    .handler(thread::create_thread)
                    .handler(thread::list_threads)
                    .handler(thread::get_thread)
                    .handler(thread::make_post)
                    .handler(thread::delete_thread),
            ),
    )
}

pub struct SessionUser(User);
impl FromRequestParts<()> for SessionUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, &())
            .await
            .map_err(|_| ApiError::server_error("Missing session layer"))?;
        let user = rorm::query(Database::global(), User)
            .condition(
                User.id.equals(
                    session
                        .get::<i64>("user_id")
                        .await?
                        .ok_or_else(|| ApiError::bad_request("Please login first"))?,
                ),
            )
            .one()
            .await?;
        Ok(Self(user))
    }
}
