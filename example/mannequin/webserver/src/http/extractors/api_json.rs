//! Alternative for [`axum::Json`] which produces our [`ApiError`] in case of failure

use axum::extract::FromRequest;
use axum::response::IntoResponse;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use swaggapi::as_responses::AsResponses;
use swaggapi::handler_argument::HandlerArgument;
use swaggapi::handler_argument::ShouldBeHandlerArgument;
use swaggapi::internals::SchemaGenerator;
use swaggapi::re_exports::openapiv3::Parameter;
use swaggapi::re_exports::openapiv3::RequestBody;
use swaggapi::re_exports::openapiv3::Responses;

use crate::http::common::errors::ApiError;

/// Alternative for [`axum::Json`] which produces our [`ApiError`] in case of failure
#[derive(Copy, Clone, Default, Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(pub T);

// We implement `IntoResponse` for our extractor so it can be used as a response
impl<T: Serialize> IntoResponse for ApiJson<T> {
    fn into_response(self) -> axum::response::Response {
        let Self(value) = self;
        // TODO
        axum::Json(value).into_response()
    }
}

impl<T: Serialize + JsonSchema> AsResponses for ApiJson<T> {
    fn responses(gen: &mut SchemaGenerator) -> Responses {
        axum::Json::<T>::responses(gen)
    }
}

impl<T> ShouldBeHandlerArgument for ApiJson<T> {}
impl<T: DeserializeOwned + JsonSchema> HandlerArgument for ApiJson<T> {
    fn request_body(gen: &mut SchemaGenerator) -> Option<RequestBody> {
        axum::Json::<T>::request_body(gen)
    }
    fn parameters(gen: &mut SchemaGenerator, path: &[&str]) -> Vec<Parameter> {
        axum::Json::<T>::parameters(gen, path)
    }
}
