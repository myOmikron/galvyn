use galvyn::core::re_exports::axum::extract::Path;
use galvyn::core::re_exports::schemars;
use galvyn::core::re_exports::schemars::JsonSchema;
use galvyn::core::session::Session;
use galvyn::core::stuff::api_error::ApiError;
use galvyn::core::stuff::api_error::ApiResult;
use galvyn::core::stuff::api_json::ApiJson;
use galvyn::core::Module;
use galvyn::get;
use galvyn::post;
use galvyn::put;
use galvyn::rorm;
use galvyn::rorm::and;
use galvyn::rorm::fields::types::MaxStr;
use galvyn::rorm::Database;
use serde::Deserialize;
use serde::Serialize;

use crate::handler::SessionUser;
use crate::models::user::NewUser;
use crate::models::user::User;
use crate::models::user::UserRole;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RegisterRequest {
    pub username: MaxStr<255>,
    pub password: String,
}
#[post("/register")]
pub async fn register(
    session: Session,
    ApiJson(request): ApiJson<RegisterRequest>,
) -> ApiResult<()> {
    let mut tx = Database::global().start_transaction().await?;
    if rorm::query(&mut tx, User.id)
        .condition(User.username.equals(&*request.username))
        .optional()
        .await?
        .is_some()
    {
        return Err(ApiError::bad_request("Username already taken"));
    }
    let id = rorm::insert(&mut tx, User)
        .return_primary_key()
        .single(&NewUser {
            username: request.username,
            password: request.password,
            role: UserRole::User,
        })
        .await?;
    session.insert("user_id", &id).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[post("/login")]
pub async fn login(session: Session, ApiJson(request): ApiJson<LoginRequest>) -> ApiResult<()> {
    if let Some(id) = rorm::query(Database::global(), User.id)
        .condition(and![
            User.username.equals(&request.username),
            User.password.equals(&request.password)
        ])
        .optional()
        .await?
    {
        session.insert("user_id", &id).await?;
        Ok(())
    } else {
        Err(ApiError::bad_request("Invalid password or username"))
    }
}

#[put("/logout")]
pub async fn logout(session: Session) -> ApiResult<()> {
    session.flush().await?;
    Ok(())
}

#[put("/delete")]
pub async fn delete(SessionUser(user): SessionUser, session: Session) -> ApiResult<()> {
    rorm::delete(Database::global(), User)
        .condition(User.id.equals(user.id))
        .await?;
    session.flush().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ProfileResponse {
    pub username: String,
    pub role: String,
    pub posts: i64,
}
#[get("/profile/{username}")]
pub async fn profile(
    SessionUser(_user): SessionUser,
    Path(username): Path<String>,
) -> ApiResult<ApiJson<ProfileResponse>> {
    let mut tx = Database::global().start_transaction().await?;
    let (role,) = rorm::query(&mut tx, (User.role,))
        .condition(User.username.equals(&username))
        .optional()
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown username"))?;
    let posts = rorm::query(&mut tx, User.posts.uuid.count())
        .condition(User.username.equals(&username))
        .one()
        .await?;
    tx.commit().await?;
    Ok(ApiJson(ProfileResponse {
        username,
        role: role.to_string(),
        posts,
    }))
}
