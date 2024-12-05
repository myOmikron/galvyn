//! Utilities for working with [`users::schema`](super::schema)

use rorm::db::Executor;
use rorm::prelude::ForeignModelByField;
use rorm::update;
use rorm::FieldAccess;
use rorm::Model;
use tower_sessions::Session;
use uuid::Uuid;

use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::handler_frontend::users::schema::FullUser;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::http::session_keys::SESSION_USER;
use crate::models;
use crate::models::User;
use crate::models::UserRole;

/// Construct the `UserPermissions` schema from a populated `User` model.
///
/// Errors:
/// - if `user.groups` is not populated
/// - if `user.role` is `UserRole::Customer` but `user.customers` is not populated
#[track_caller]
pub fn get_user_permissions(user: &User) -> ApiResult<UserPermissions> {
    Ok(match user.role.key().parse()? {
        UserRole::Administrator => UserPermissions::Administrator,
        UserRole::Internal => UserPermissions::Internal,
    })
}

/// Converts the populated `User` model into a `FullUser` schema.
///
/// Errors:
/// - if `user.groups` is not populated
/// - if `user.role` is `UserRole::Customer` but `user.customers` is not populated
#[track_caller]
pub fn new_full_user(user: User) -> ApiResult<FullUser> {
    Ok(FullUser {
        permissions: get_user_permissions(&user)?,
        uuid: user.uuid,
        mail: user.mail,
        display_name: user.display_name,
        preferred_lang: user.preferred_lang.parse()?,
    })
}

/// Sets the user to logged in after completing an accept option
pub async fn set_logged_in(
    executor: impl Executor<'_>,
    session: &Session,
    user_uuid: Uuid,
) -> ApiResult<()> {
    session.insert(SESSION_USER, user_uuid).await?;
    session.save().await?;

    let Some(id) = session.id() else {
        return Err(ApiError::new_internal_server_error("No ID in session"));
    };
    update!(executor, models::Session)
        .condition(models::Session::F.id.equals(id.to_string()))
        .set(
            models::Session::F.user,
            Some(ForeignModelByField::Key(user_uuid)),
        )
        .exec()
        .await?;

    Ok(())
}
