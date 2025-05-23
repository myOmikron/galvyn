//! Alternative for [`axum::Json`] which produces our [`ApiError`] in case of failure

use axum::Json;
use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use schemars::schema::Schema;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::handler::request_body::RequestBody;
use crate::handler::request_body::ShouldBeRequestBody;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;
use crate::schema_generator::SchemaGenerator;
use crate::stuff::api_error::ApiError;

/// Alternative for [`Json`] which produces our [`ApiError`] in case of failure
#[derive(Copy, Clone, Default, Debug)]
pub struct ApiJson<T>(pub T);

impl<S, T: DeserializeOwned> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = <Bytes as FromRequest<S>>::from_request(req, state)
            .await
            .map_err(|err| {
                ApiError::server_error("Failed to buffer request body").with_source(err)
            })?;
        serde_json::from_slice(bytes.as_ref())
            .map(Self)
            .map_err(|error| {
                ApiError::bad_request("Failed to deserialize request body").with_source(error)
            })
    }
}

impl<T: Serialize> IntoResponse for ApiJson<T> {
    fn into_response(self) -> axum::response::Response {
        // Use a small initial capacity of 128 bytes like serde_json::to_vec
        // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
        let mut buf = Vec::with_capacity(128);
        match serde_json::to_writer(&mut buf, &self.0) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                buf,
            )
                .into_response(),
            Err(err) => ApiError::server_error("Failed to serialize response body")
                .with_source(err)
                .into_response(),
        }
    }
}

impl<T> ShouldBeResponseBody for ApiJson<T> {}
impl<T> ResponseBody for ApiJson<T>
where
    Json<T>: ResponseBody,
{
    fn body(
        generator: &mut SchemaGenerator,
    ) -> Vec<(StatusCode, Option<(mime::Mime, Option<Schema>)>)> {
        Json::<T>::body(generator)
    }
}

impl<T> ShouldBeRequestBody for ApiJson<T> {}
impl<T> RequestBody for ApiJson<T>
where
    Json<T>: RequestBody,
{
    fn body(generator: &mut SchemaGenerator) -> (mime::Mime, Option<Schema>) {
        Json::<T>::body(generator)
    }
}
