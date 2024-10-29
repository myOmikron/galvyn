//! Collections of some utilities

use axum::async_trait;
use axum::extract::FromRequest;
use axum::extract::Request;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;

/// JSON Extractor / Response which doesn't require [`JsonSchema`](schemars::JsonSchema)
///
/// Just think of this type as the `Json<T>` in the framework of your choice
/// and use it if you don't want to bother to make `T` implement [`JsonSchema`](schemars::JsonSchema).
#[derive(Copy, Clone, Debug)]
pub struct SchemalessJson<T>(pub T);

#[async_trait]
impl<T, S: Sync> FromRequest<S> for SchemalessJson<T>
where
    Json<T>: FromRequest<S>,
{
    type Rejection = <Json<T> as FromRequest<S>>::Rejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        <Json<T> as FromRequest<S>>::from_request(req, state)
            .await
            .map(|Json(t)| SchemalessJson(t))
    }
}

impl<T> IntoResponse for SchemalessJson<T>
where
    Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}
