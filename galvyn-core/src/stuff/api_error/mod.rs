//! This module holds the errors and the error conversion for handlers
//! that are returned from handlers

use std::any::TypeId;
use std::error::Error;
use std::fmt;
use std::ops::Deref;
use std::panic::Location;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use schemars::JsonSchema;
use schemars::schema::Schema;
use serde::Serialize;

pub use self::aggregator::FormErrors;
use crate::handler::context::EndpointContext;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;
use crate::stuff::api_error::core::CoreApiError;
use crate::stuff::api_json::ApiJson;
use crate::stuff::schema::FormErrorResponse;
use crate::stuff::schema::Never;

mod aggregator;
pub mod core;

/// A type alias that includes the ApiError
pub type ApiResult<T, E = Never> = Result<T, ApiError<E>>;

/// The common error that is returned from the handlers
#[derive(Debug)]
pub enum ApiError<E = Never> {
    CoreApiError(CoreApiError),
    FormError(E),
}
impl<E> ApiError<E> {
    /// Constructs a new `ApiError` with [`ApiErrorStatusCode::BadRequest`]
    #[track_caller]
    pub fn bad_request(context: &'static str) -> Self {
        Self::CoreApiError(CoreApiError::bad_request(context))
    }

    /// Constructs a new `ApiError` with [`ApiErrorStatusCode::ServerError`]
    #[track_caller]
    pub fn server_error(context: &'static str) -> Self {
        Self::CoreApiError(CoreApiError::server_error(context))
    }

    /// Constructs a new `ApiError` with [`ApiErrorStatusCode::Unauthorized`]
    #[track_caller]
    pub fn unauthorized(context: &'static str) -> Self {
        Self::CoreApiError(CoreApiError::unauthorized(context))
    }

    /// Adds a source to the `ApiError`
    ///
    /// # Panics
    /// if called on a `ApiError::FormError(_)`
    #[track_caller]
    pub fn with_source(self, source: impl Error + Send + Sync + 'static) -> Self {
        self.map_api_error(|core| core.with_source(source))
    }

    /// Adds a source to the `ApiError`
    ///
    /// # Panics
    /// if called on a `ApiError::FormError(_)`
    #[track_caller]
    pub fn with_boxed_source(self, source: Box<dyn Error + Send + Sync + 'static>) -> Self {
        self.map_api_error(|core| core.with_boxed_source(source))
    }

    /// Adds a location to the `ApiError`
    ///
    /// Normally the location which is added automatically is enough.
    ///
    /// # Panics
    /// if called on a `ApiError::FormError(_)`
    #[track_caller]
    pub fn with_manual_location(self, location: &'static Location<'static>) -> Self {
        self.map_api_error(|core| core.with_manual_location(location))
    }

    /// Creates a closure for wrapping any error into an `ApiError::server_error`
    ///
    /// This is just a less noisy shorthand for `|error| ApiError::server_error("...").with_source(error)`.
    #[track_caller]
    pub fn map_server_error<F: Error + Send + Sync + 'static>(
        context: &'static str,
    ) -> impl Fn(F) -> Self {
        let location = Location::caller();
        move |error| {
            Self::server_error(context)
                .with_source(error)
                .with_manual_location(location)
        }
    }

    /// Emit a tracing event `error!` or `debug!` describing the `ApiError`
    ///
    /// `ApiError::FormError(_)` won't log anything.
    pub fn emit_tracing_event(&self) {
        match self {
            ApiError::CoreApiError(core) => core.emit_tracing_event(),
            ApiError::FormError(_) => {}
        }
    }

    #[track_caller]
    fn map_api_error(self, map: impl FnOnce(CoreApiError) -> CoreApiError) -> Self {
        match self {
            ApiError::CoreApiError(x) => ApiError::CoreApiError(map(x)),
            ApiError::FormError(_) => panic!(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::CoreApiError(core) => fmt::Display::fmt(core, f),
            ApiError::FormError(never) => match *never {},
        }
    }
}
impl Error for ApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApiError::CoreApiError(core) => Error::source(core),
            ApiError::FormError(never) => match *never {},
        }
    }
}
impl Deref for ApiError {
    type Target = CoreApiError;

    fn deref(&self) -> &Self::Target {
        match self {
            ApiError::CoreApiError(core) => core,
            ApiError::FormError(never) => match *never {},
        }
    }
}

impl<E: Serialize> IntoResponse for ApiError<E> {
    fn into_response(self) -> Response {
        self.emit_tracing_event();
        match self {
            ApiError::CoreApiError(core) => core.into_response(),
            ApiError::FormError(error) => ApiJson(FormErrorResponse {
                error,
                result: Default::default(),
            })
            .into_response(),
        }
    }
}

impl<E> ShouldBeResponseBody for ApiError<E> {}
impl<E: JsonSchema + 'static> ResponseBody for ApiError<E> {
    fn body(ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(mime::Mime, Option<Schema>)>)> {
        let mut bodies = Vec::new();
        if TypeId::of::<E>() != TypeId::of::<Never>() {
            let form_error = ctx.generator.generate::<FormErrorResponse<E>>();
            bodies.extend([(
                StatusCode::OK,
                Some((mime::APPLICATION_JSON, Some(form_error))),
            )]);
        }
        bodies.extend(CoreApiError::body(ctx));
        bodies
    }
}

impl<E, F> From<F> for ApiError<E>
where
    CoreApiError: From<F>,
{
    fn from(value: F) -> Self {
        Self::CoreApiError(value.into())
    }
}
