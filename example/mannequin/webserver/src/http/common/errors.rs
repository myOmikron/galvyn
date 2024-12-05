//! This module holds the errors and the error conversion for handlers
//! that are returned from handlers

use std::error::Error;
use std::panic::Location;
use std::time::SystemTimeError;

use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use swaggapi::as_responses::simple_responses;
use swaggapi::as_responses::AsResponses;
use swaggapi::as_responses::SimpleResponse;
use swaggapi::internals::SchemaGenerator;
use swaggapi::re_exports::mime;
use swaggapi::re_exports::openapiv3;
use swaggapi::re_exports::openapiv3::MediaType;
use swaggapi::re_exports::openapiv3::Responses;
use thiserror::Error;
use tracing::error;
use webauthn_rs::prelude::WebauthnError;

use crate::http::common::schemas::ApiErrorResponse;
use crate::http::common::schemas::ApiStatusCode;
use crate::utils::checked_string;
use crate::utils::totp::TotpFromError;

/// A type alias that includes the ApiError
pub type ApiResult<T> = Result<T, ApiError>;

/// The common error that is returned from the handlers
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum ApiError {
    #[error("Unauthenticated")]
    Unauthenticated,

    #[error("Missing privileges")]
    MissingPrivileges,

    #[error("Bad request")]
    BadRequest,

    #[error("Invalid json received: {0}")]
    InvalidJson(#[from] JsonRejection),

    #[error("An internal server error occurred")]
    InternalServerError {
        location: &'static Location<'static>,
        source: DynError,
    },
}
type DynError = Box<dyn Error + Send + Sync + 'static>;

impl ApiError {
    /// Ad-hoc constructor for an internal server error
    /// where the causing `error` doesn't have a proper `From` impl.
    // This function takes `impl Into<Box<dyn Error>>` instead of `impl Error` to also support strings
    #[track_caller]
    pub fn new_internal_server_error(error: impl Into<DynError>) -> Self {
        Self::InternalServerError {
            location: Location::caller(),
            source: error.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            ApiError::Unauthenticated => (
                ApiStatusCode::Unauthenticated,
                "Unauthenticated".to_string(),
            ),
            ApiError::BadRequest => (ApiStatusCode::BadRequest, "Bad Request".to_string()),
            ApiError::MissingPrivileges => (
                ApiStatusCode::MissingPrivileges,
                "Missing Privileges".to_string(),
            ),
            ApiError::InvalidJson(msg) => (ApiStatusCode::InvalidJson, msg.to_string()),
            ApiError::InternalServerError { location, source } => {
                error!(
                    error.display = %source,
                    error.debug = ?source,
                    error.file = location.file(),
                    error.line = location.line(),
                    error.column = location.column(),
                    "Internal server error",
                );
                (
                    ApiStatusCode::InternalServerError,
                    "Internal server error occurred".to_string(),
                )
            }
        };

        let res = (
            if (status_code as u16) < 2000 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            },
            Json(ApiErrorResponse {
                status_code,
                message,
            }),
        );

        res.into_response()
    }
}

impl AsResponses for ApiError {
    fn responses(gen: &mut SchemaGenerator) -> Responses {
        let media_type = Some(MediaType {
            schema: Some(gen.generate::<ApiErrorResponse>()),
            ..Default::default()
        });

        simple_responses([
            SimpleResponse {
                status_code: openapiv3::StatusCode::Code(400),
                mime_type: mime::APPLICATION_JSON,
                description: "Client side error".to_string(),
                media_type: media_type.clone(),
            },
            SimpleResponse {
                status_code: openapiv3::StatusCode::Code(500),
                mime_type: mime::APPLICATION_JSON,
                description: "Server side error".to_string(),
                media_type,
            },
        ])
    }
}

/// Simple macro to reduce the noise of several identical `From` implementations
///
/// It takes a list of error types
/// which are supposed to be convertable into an [`ApiError::InternalServerError`] simplicity.
macro_rules! impl_into_internal_server_error {
    ($($error:ty,)*) => {$(
        impl From<$error> for ApiError {
            #[track_caller]
            fn from(value: $error) -> Self {
                Self::new_internal_server_error(value)
            }
        }
    )+};
}
impl_into_internal_server_error!(
    rorm::Error,
    argon2::password_hash::Error,
    tower_sessions::session::Error,
    strum::ParseError,
    checked_string::ConstraintsViolated,
    SystemTimeError,
    TotpFromError,
    WebauthnError,
);
