//! Admin handlers for user invites

use axum::extract::Path;
use futures::TryStreamExt;
use rorm::query;
use rorm::FieldAccess;
use rorm::Model;
use swaggapi::delete;
use swaggapi::get;
use swaggapi::post;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::common::schemas::FormResult;
use crate::http::common::schemas::List;
use crate::http::common::schemas::SingleUuid;
use crate::http::extractors::api_json::ApiJson;
use crate::http::handler_frontend::user_invites::schema::CreateUserInviteErrors;
use crate::http::handler_frontend::user_invites::schema::CreateUserInviteMailError;
use crate::http::handler_frontend::user_invites::schema::CreateUserInviteRequest;
use crate::http::handler_frontend::user_invites::schema::SimpleUserInvite;
use crate::http::handler_frontend::user_invites::utils::new_simple_user_invite;
use crate::models::CreateUserInviteError;
use crate::models::UserInvite;

/// Invite a new (local) user
#[post("/")]
pub async fn create_user_invite(
    ApiJson(request): ApiJson<CreateUserInviteRequest>,
) -> ApiResult<ApiJson<FormResult<SimpleUserInvite, CreateUserInviteErrors>>> {
    let invite = match UserInvite::create(
        &GLOBAL.db,
        request.mail,
        request.display_name,
        request.preferred_lang,
        request.permissions,
    )
    .await
    {
        Ok(invite) => invite,
        Err(CreateUserInviteError::AlreadyUser) => {
            return Ok(ApiJson(FormResult::err(CreateUserInviteErrors {
                mail: Some(CreateUserInviteMailError::AlreadyUser),
            })))
        }
        Err(CreateUserInviteError::AlreadyInvited) => {
            return Ok(ApiJson(FormResult::err(CreateUserInviteErrors {
                mail: Some(CreateUserInviteMailError::AlreadyInvited),
            })))
        }
        Err(CreateUserInviteError::Database(error)) => return Err(error.into()),
    };
    Ok(ApiJson(FormResult::ok(new_simple_user_invite(invite)?)))
}

/// Retrieve all outstanding invites (expired or not)
#[get("/")]
pub async fn get_all_user_invites() -> ApiResult<ApiJson<List<SimpleUserInvite>>> {
    let list = query!(&GLOBAL.db, UserInvite)
        .stream()
        .err_into::<ApiError>()
        .and_then(|invite| async move { new_simple_user_invite(invite) })
        .try_collect()
        .await?;
    Ok(ApiJson(List { list }))
}

/// Delete an outstanding invite
#[delete("/:uuid")]
pub async fn delete_user_invite(Path(SingleUuid { uuid }): Path<SingleUuid>) -> ApiResult<()> {
    rorm::delete!(&GLOBAL.db, UserInvite)
        .condition(UserInvite::F.uuid.equals(uuid))
        .await?;
    Ok(())
}
