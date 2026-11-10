use axum::extract::FromRequestParts;
use axum::http;
use http::request::Parts;

use crate::session::Session;

impl<S> FromRequestParts<S> for Session
where
    S: Sync + Send,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Session>().cloned().ok_or((
            http::StatusCode::INTERNAL_SERVER_ERROR,
            "Can't extract session. Is `SessionManagerLayer` enabled?",
        ))
    }
}
