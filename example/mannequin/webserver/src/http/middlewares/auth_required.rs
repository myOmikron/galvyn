//! Authentication required middleware

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tower_sessions::Session;
use uuid::Uuid;

use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::session_keys::SESSION_USER;

/// Checks the session if the [SESSION_USER] is present which will be the indicator
/// if the user is logged-in
pub async fn auth_required(session: Session, req: Request, next: Next) -> ApiResult<Response> {
    session
        .get::<Uuid>(SESSION_USER)
        .await?
        .ok_or(ApiError::Unauthenticated)?;

    Ok(next.run(req).await)
}
