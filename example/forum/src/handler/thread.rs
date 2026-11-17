use std::collections::HashMap;

use futures_util::TryStreamExt;
use galvyn::core::re_exports::axum::extract::Path;
use galvyn::core::re_exports::schemars;
use galvyn::core::re_exports::schemars::JsonSchema;
use galvyn::core::stuff::api_error::ApiError;
use galvyn::core::stuff::api_error::ApiResult;
use galvyn::core::stuff::api_json::ApiJson;
use galvyn::core::stuff::schema::SchemaDateTime;
use galvyn::core::Module;
use galvyn::delete;
use galvyn::get;
use galvyn::post;
use galvyn::rorm;
use galvyn::rorm::and;
use galvyn::rorm::fields::types::CascadingForeignModelByField;
use galvyn::rorm::fields::types::MaxStr;
use galvyn::rorm::prelude::ForeignModelByField;
use galvyn::rorm::Database;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::handler::SessionUser;
use crate::models::post::NewPost;
use crate::models::post::Post;
use crate::models::thread::NewThread;
use crate::models::thread::Thread;
use crate::models::user::User;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CreateThreadRequest {
    pub name: String,
}

#[post("/create")]
pub async fn create_thread(
    SessionUser(_user): SessionUser,
    ApiJson(request): ApiJson<CreateThreadRequest>,
) -> ApiResult<ApiJson<String>> {
    let mut tx = Database::global().start_transaction().await?;
    let identifier = request.name.to_ascii_lowercase();
    if rorm::query(&mut tx, Thread)
        .condition(Thread.identifier.equals(&identifier))
        .optional()
        .await?
        .is_some()
    {
        return Err(ApiError::bad_request("Please choose another name"));
    }
    rorm::insert(&mut tx, Thread)
        .single(&NewThread {
            identifier: identifier.clone(),
            name: request.name,
        })
        .await?;
    tx.commit().await?;
    Ok(ApiJson(identifier))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ListResponse {
    pub threads: Vec<ListResponseItem>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ListResponseItem {
    pub identifier: String,
    pub name: String,
    pub opened_at: SchemaDateTime,
}

#[get("/list")]
pub async fn list_threads(SessionUser(_user): SessionUser) -> ApiResult<ApiJson<ListResponse>> {
    let threads = rorm::query(Database::global(), Thread).all().await?;
    Ok(ApiJson(ListResponse {
        threads: threads
            .into_iter()
            .map(|x| ListResponseItem {
                identifier: x.identifier,
                name: x.name,
                opened_at: SchemaDateTime(x.opened_at),
            })
            .collect(),
    }))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GetResponse {
    pub identifier: String,
    pub name: String,
    pub opened_at: SchemaDateTime,
    pub posts: Vec<ThreadPost>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ThreadPost {
    pub uuid: Uuid,
    pub user: Option<MaxStr<255>>,
    pub message: String,
    pub posted_at: SchemaDateTime,
    pub replies: i64,
}

#[get("/get/{identifier}")]
pub async fn get_thread(Path(thread): Path<String>) -> ApiResult<ApiJson<GetResponse>> {
    let mut tx = Database::global().start_transaction().await?;

    let (name, opened_at) = rorm::query(&mut tx, (Thread.name, Thread.opened_at))
        .condition(Thread.identifier.equals(&thread))
        .optional()
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown thread"))?;

    let users = rorm::query(&mut tx, (User.id, User.username))
        .stream()
        .try_collect::<HashMap<_, _>>()
        .await?;

    let posts: Vec<_> = rorm::query(
        &mut tx,
        (Post.uuid, Post.message, Post.user, Post.posted_at),
    )
    .condition(Post.thread.equals(&thread))
    .order_asc(Post.posted_at)
    .stream()
    .map_ok(|(uuid, message, user, posted_at)| ThreadPost {
        uuid,
        user: user.map(|ForeignModelByField(id)| users[&id].clone()),
        message: message.into_inner(),
        posted_at: SchemaDateTime(posted_at),
        replies: 0,
    })
    .try_collect()
    .await?;

    tx.commit().await?;
    Ok(ApiJson(GetResponse {
        identifier: thread,
        name,
        opened_at: SchemaDateTime(opened_at),
        posts,
    }))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MakePostRequest {
    pub message: String,
    pub reply_to: Option<Uuid>,
}
#[post("/posts/{identifier}")]
pub async fn make_post(
    SessionUser(user): SessionUser,
    Path(thread): Path<String>,
    ApiJson(request): ApiJson<MakePostRequest>,
) -> ApiResult<()> {
    let mut tx = Database::global().start_transaction().await?;

    rorm::query(&mut tx, Thread.identifier)
        .condition(Thread.identifier.equals(&thread))
        .optional()
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown thread"))?;

    if let Some(reply_to) = request.reply_to {
        rorm::query(&mut tx, Post.uuid)
            .condition(and![
                Post.uuid.equals(reply_to),
                Post.thread.equals(&thread),
            ])
            .optional()
            .await?
            .ok_or_else(|| ApiError::bad_request("Unknown post"))?;
    }

    rorm::insert(&mut tx, Post)
        .return_nothing()
        .single(&NewPost {
            uuid: Uuid::new_v4(),
            message: MaxStr::new(request.message)
                .map_err(|_| ApiError::bad_request("Post's message is too long"))?,
            user: Some(ForeignModelByField(user.id)),
            thread: CascadingForeignModelByField(thread),
            reply_to: request.reply_to.map(ForeignModelByField),
        })
        .await?;

    tx.commit().await?;
    Ok(())
}

#[delete("/delete/{identifier}")]
pub async fn delete_thread(
    SessionUser(_user): SessionUser,
    Path(identifier): Path<String>,
) -> ApiResult<()> {
    rorm::delete(Database::global(), Thread)
        .condition(Thread.identifier.equals(&identifier))
        .await?;
    Ok(())
}
