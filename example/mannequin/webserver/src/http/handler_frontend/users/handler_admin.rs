//! The handler for the users

use axum::extract::Path;
use rorm::query;
use swaggapi::delete;
use swaggapi::get;
use swaggapi::put;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiResult;
use crate::http::common::schemas::List;
use crate::http::common::schemas::SingleUuid;
use crate::http::extractors::api_json::ApiJson;
use crate::http::handler_frontend::users::schema::FullUser;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::http::handler_frontend::users::utils::new_full_user;
use crate::models::User;

/// Retrieves an unordered, unsorted list of all users
#[get("/")]
pub async fn get_all_users() -> ApiResult<ApiJson<List<FullUser>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let users = query!(&mut tx, User).all().await?;

    tx.commit().await?;
    Ok(ApiJson(List {
        list: users
            .into_iter()
            .map(new_full_user)
            .collect::<Result<_, _>>()?,
    }))
}

/// Overwrites a user's permissions
#[put("/:uuid/permissions")]
pub async fn set_user_permissions(
    Path(SingleUuid { uuid }): Path<SingleUuid>,
    ApiJson(new_permissions): ApiJson<UserPermissions>,
) -> ApiResult<()> {
    User::set_permissions(&GLOBAL.db, uuid, new_permissions).await?;
    Ok(())
}

/// Deletes a user
#[delete("/:uuid")]
pub async fn delete_user(Path(SingleUuid { uuid }): Path<SingleUuid>) -> ApiResult<()> {
    User::delete(&GLOBAL.db, uuid).await?;
    Ok(())
}
