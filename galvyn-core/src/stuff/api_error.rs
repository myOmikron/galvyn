//! This module holds the errors and the error conversion for handlers
//! that are returned from handlers

use std::any::TypeId;
use std::error::Error;
use std::fmt;
use std::ops::Deref;
use std::ops::DerefMut;
use std::panic::Location;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
#[cfg(feature = "opentelemetry")]
use opentelemetry::trace::TraceId;
use rorm::crud::update::UpdateBuilder;
use schemars::JsonSchema;
use schemars::schema::Schema;
use serde::Serialize;
use thiserror::Error;
use tracing::debug;
use tracing::error;

use crate::handler::context::EndpointContext;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;
use crate::stuff::api_json::ApiJson;
use crate::stuff::schema::ApiErrorResponse;
use crate::stuff::schema::ApiStatusCode;
use crate::stuff::schema::ErrorConstant;
use crate::stuff::schema::FormErrorResponse;
use crate::stuff::schema::Never;

/// A type alias that includes the ApiError
pub type ApiResult<T, E = Never> = Result<T, ApiError<E>>;

pub enum ApiError<E = Never> {
    ApiError(InnerApiError),
    FormError(E),
}

#[derive(Default)]
pub struct FormErrors<E> {
    inner: E,
    modified: bool,
}

impl<E: Default> FormErrors<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(self) -> ApiResult<(), E> {
        if self.modified {
            Err(ApiError::FormError(self.inner))
        } else {
            Ok(())
        }
    }

    pub fn fail<T>(self) -> ApiResult<T, E> {
        Err(ApiError::FormError(self.inner))
    }
}

impl<E> Deref for FormErrors<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for FormErrors<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.modified = true;
        &mut self.inner
    }
}

/// The common error that is returned from the handlers
#[derive(Debug, Error)]
struct InnerApiError {
    /// Rough indication of the error reason (exposed to frontend)
    pub code: ApiStatusCode,

    /// An arbitrary string literal describing the error
    pub context: Option<&'static str>,

    /// Location where the error originated from
    pub location: &'static Location<'static>,

    /// The error's underlying source
    pub source: Option<Box<dyn Error + Send + Sync + 'static>>,

    /// ID of the opentelemetry trace this error originated in
    #[cfg(feature = "opentelemetry")]
    pub trace_id: TraceId,
}

impl fmt::Display for InnerApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            ApiStatusCode::Unauthenticated
            | ApiStatusCode::BadRequest
            | ApiStatusCode::InvalidJson
            | ApiStatusCode::MissingPrivileges => write!(f, "Bad Request")?,
            ApiStatusCode::InternalServerError => write!(f, "Server Error")?,
        }
        if let Some(context) = self.context {
            write!(f, " '{context}'")?;
        }
        if let Some(source) = &self.source {
            write!(f, " cause by '{source}'")?;
        }
        write!(f, " at '{}'", self.location)
    }
}

impl<T> ApiError<T> {
    /// Constructs a new `ApiError`
    #[track_caller]
    pub fn new(code: ApiStatusCode, context: &'static str) -> Self {
        Self::ApiError(InnerApiError {
            code,
            context: Some(context),
            location: Location::caller(),
            source: None,
            #[cfg(feature = "opentelemetry")]
            trace_id: Self::get_trace_id(),
        })
    }

    /// Constructs a new `ApiError` with [`ApiStatusCode::BadRequest`]
    #[track_caller]
    pub fn bad_request(context: &'static str) -> Self {
        Self::new(ApiStatusCode::BadRequest, context)
    }

    /// Constructs a new `ApiError` with [`ApiStatusCode::InternalServerError`]
    #[track_caller]
    pub fn server_error(context: &'static str) -> Self {
        Self::new(ApiStatusCode::InternalServerError, context)
    }

    /// Adds a source to the `ApiError`
    pub fn with_source(self, source: impl Error + Send + Sync + 'static) -> Self {
        self.with_boxed_source(source.into())
    }

    /// Adds a source to the `ApiError`
    pub fn with_boxed_source(self, source: Box<dyn Error + Send + Sync + 'static>) -> Self {
        match self {
            ApiError::ApiError(mut error) => {
                error.source = Some(source);
                ApiError::ApiError(error)
            }
            ApiError::FormError(_) => {
                panic!();
            }
        }
    }

    /// Creates a closure for wrapping any error into an `ApiError::server_error`
    ///
    /// This is just a less noisy shorthand for `|error| ApiError::server_error("...").with_source(error)`.
    #[track_caller]
    pub fn map_server_error<E: Error + Send + Sync + 'static>(
        context: &'static str,
    ) -> impl Fn(E) -> Self {
        move |error| Self::server_error(context).with_source(error)
    }

    /// Emit a tracing event `error!` or `debug!` describing the `ApiError`
    pub fn emit_tracing_event(&self) {
        let Self::ApiError(InnerApiError {
            code,
            context,
            location,
            source,
            #[cfg(feature = "opentelemetry")]
                trace_id: _, // The log message will hopefully be emitted in the same span
        }) = &self
        else {
            return;
        };

        match code {
            ApiStatusCode::Unauthenticated
            | ApiStatusCode::BadRequest
            | ApiStatusCode::InvalidJson
            | ApiStatusCode::MissingPrivileges => {
                debug!(
                    error.code = ?code,
                    error.context = context,
                    error.file = location.file(),
                    error.line = location.line(),
                    error.column = location.column(),
                    error.display = source.as_ref().map(tracing::field::display),
                    error.debug = source.as_ref().map(tracing::field::debug),
                    "Client error"
                );
            }
            ApiStatusCode::InternalServerError => {
                error!(
                    error.code = ?code,
                    error.context = context,
                    error.file = location.file(),
                    error.line = location.line(),
                    error.column = location.column(),
                    error.display = source.as_ref().map(tracing::field::display),
                    error.debug = source.as_ref().map(tracing::field::debug),
                    "Server error"
                );
            }
        }
    }

    /// Adds a location to the `ApiError`
    ///
    /// Normally the location added automatically is enough.
    pub fn with_manual_location(self, location: &'static Location<'static>) -> Self {
        match self {
            ApiError::ApiError(mut error) => {
                error.location = location;
                ApiError::ApiError(error)
            }
            ApiError::FormError(_) => {
                panic!();
            }
        }
    }

    /// Retrieves the current span's trace id
    ///
    /// This little helper can be used to construct an `ApiError` with a literal.
    #[cfg(feature = "opentelemetry")]
    pub fn get_trace_id() -> TraceId {
        use opentelemetry::trace::TraceContextExt;
        use tracing::Span;
        use tracing_opentelemetry::OpenTelemetrySpanExt;

        Span::current().context().span().span_context().trace_id()
    }
}

impl<T: Serialize> IntoResponse for ApiError<T> {
    fn into_response(self) -> Response {
        self.emit_tracing_event();

        match self {
            ApiError::ApiError(error) => (
                match error.code {
                    ApiStatusCode::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
                    ApiStatusCode::Unauthenticated => StatusCode::UNAUTHORIZED,
                    _ => StatusCode::BAD_REQUEST,
                },
                ApiJson(ApiErrorResponse {
                    status_code: error.code,
                    message: match error.code {
                        ApiStatusCode::Unauthenticated => "Unauthenticated",
                        ApiStatusCode::BadRequest => "Bad request",
                        ApiStatusCode::InvalidJson => "Invalid json",
                        ApiStatusCode::MissingPrivileges => "Missing privileges",
                        ApiStatusCode::InternalServerError => "Internal server error",
                    }
                    .to_string(),
                    #[cfg(feature = "opentelemetry")]
                    trace_id: error.trace_id.to_string(),
                }),
            )
                .into_response(),
            ApiError::FormError(error) => (
                StatusCode::OK,
                ApiJson(FormErrorResponse {
                    error,
                    result: ErrorConstant::Err,
                }),
            )
                .into_response(),
        }
    }
}

impl<T> ShouldBeResponseBody for ApiError<T> {}
impl<T: JsonSchema + 'static> ResponseBody for ApiError<T> {
    fn body(ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(mime::Mime, Option<Schema>)>)> {
        let mut bodies = Vec::new();

        if TypeId::of::<T>() != TypeId::of::<Never>() {
            let form_error = ctx.generator.generate::<FormErrorResponse<T>>();
            bodies.extend([(
                StatusCode::OK,
                Some((mime::APPLICATION_JSON, Some(form_error))),
            )]);
        }

        let api_error = ctx.generator.generate::<ApiErrorResponse>();
        bodies.extend([
            (
                StatusCode::BAD_REQUEST,
                Some((mime::APPLICATION_JSON, Some(api_error.clone()))),
            ),
            (
                StatusCode::UNAUTHORIZED,
                Some((mime::APPLICATION_JSON, Some(api_error.clone()))),
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Some((mime::APPLICATION_JSON, Some(api_error))),
            ),
        ]);

        bodies
    }
}

impl<'rf, E, M, T> From<UpdateBuilder<'rf, E, M, rorm::crud::update::columns::Empty>>
    for ApiError<T>
{
    #[track_caller]
    fn from(_value: UpdateBuilder<'rf, E, M, rorm::crud::update::columns::Empty>) -> Self {
        Self::bad_request("Nothing to update")
    }
}

/// Simple macro to reduce the noise of several identical `From` implementations
///
/// It takes a list of error types
/// which are supposed to be convertable into an [`InnerApiError::server_error`] simplicity.
macro_rules! impl_into_internal_server_error {
    ($($error:ty,)*) => {$(
        impl<T> From<$error> for ApiError<T> {
            #[track_caller]
            fn from(value: $error) -> Self {
                ApiError::ApiError(InnerApiError {
                    code: ApiStatusCode::InternalServerError,
                    context: None,
                    location: Location::caller(),
                    source: Some(value.into()),
                    #[cfg(feature = "opentelemetry")]
                    trace_id: Self::get_trace_id(),
                })
            }
        }
    )+};
}
impl_into_internal_server_error!(rorm::Error, tower_sessions::session::Error, anyhow::Error,);
