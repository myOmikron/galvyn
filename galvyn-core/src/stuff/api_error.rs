//! This module holds the errors and the error conversion for handlers
//! that are returned from handlers

use std::error::Error;
use std::panic::Location;

use crate::handler::response_body::{ResponseBody, ShouldBeResponseBody};

use crate::schema_generator::SchemaGenerator;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use mime::Mime;
use schemars::schema::Schema;

use tracing::info;

/// A type alias that includes the ApiError
pub type ApiResult<T> = Result<T, DynError>;

pub struct ApiError {
    kind: ApiErrorKind,
    location: Option<&'static Location<'static>>,
    source: Option<DynError>,
}

enum ApiErrorKind {
    Client,
    Server,
}
type DynError = Box<dyn Error + Send + Sync + 'static>;

impl ApiError {
    /// Constructs a new `ApiError` which the cliebt is to be blamed for
    #[track_caller]
    pub fn client_error(error: impl Into<DynError>) -> Self {
        Self::new(error.into(), ApiErrorKind::Client)
    }

    /// Constructs a new `ApiError` which the server is to be blamed for
    #[track_caller]
    pub fn server_error(error: impl Into<DynError>) -> Self {
        Self::new(error.into(), ApiErrorKind::Server)
    }

    #[track_caller]
    fn new(source: DynError, kind: ApiErrorKind) -> Self {
        Self {
            kind,
            location: Some(Location::caller()),
            source: Some(source),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = match self.kind {
            ApiErrorKind::Client => StatusCode::BAD_REQUEST,
            ApiErrorKind::Server => StatusCode::INTERNAL_SERVER_ERROR,
        };
        info!(
            error.display = self.source.as_ref().map(tracing::field::display),
            error.debug = self.source.as_ref().map(tracing::field::debug),
            error.file = self.location.map(Location::file),
            error.line = self.location.map(Location::line),
            error.column = self.location.map(Location::column),
            "Internal server error",
        );
        status_code.into_response()
    }
}

impl ShouldBeResponseBody for ApiError {}
impl ResponseBody for ApiError {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        todo!()
    }
}
