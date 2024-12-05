//! An extractor module for extracting the uuid of the user from the session

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rorm::query;
use rorm::FieldAccess;
use rorm::Model;
use tower_sessions::Session;
use tracing::instrument;
use tracing::trace;
use uuid::Uuid;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::http::handler_frontend::users::utils::get_user_permissions;
use crate::http::session_keys::SESSION_USER;
use crate::models::User;

/// The extractor the user from the session
pub struct SessionUser {
    /// The model for the current session's user
    pub user: User,
    /// The current session user's permissions
    pub permissions: UserPermissions,
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    #[instrument(level = "trace", skip_all)]
    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = match Session::from_request_parts(req, state).await {
            Ok(session) => session,
            Err((_, error_msg)) => return Err(ApiError::new_internal_server_error(error_msg)),
        };
        let Some(user) = session.get::<Uuid>(SESSION_USER).await? else {
            trace!("{SESSION_USER} is missing in session");
            return Err(ApiError::Unauthenticated);
        };

        let mut tx = GLOBAL.db.start_transaction().await?;
        let user = query!(&mut tx, User)
            .condition(User::F.uuid.equals(user))
            .optional()
            .await?
            .ok_or(ApiError::Unauthenticated)?;
        tx.commit().await?;

        Ok(SessionUser {
            permissions: get_user_permissions(&user)?,
            user,
        })
    }
}
