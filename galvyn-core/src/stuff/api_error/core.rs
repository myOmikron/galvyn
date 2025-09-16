use std::error::Error;
use std::fmt;
use std::panic::Location;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
#[cfg(feature = "opentelemetry")]
use opentelemetry::trace::TraceId;
use rorm::crud::update::UpdateBuilder;
use schemars::schema::Schema;
use thiserror::Error;
use tracing::debug;
use tracing::error;

use crate::handler::context::EndpointContext;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;
use crate::stuff::api_json::ApiJson;
use crate::stuff::schema::ApiErrorResponse;

/// A type alias that includes the CoreApiError
pub type CoreApiResult<T> = Result<T, CoreApiError>;

/// The common error that is returned from the handlers
#[derive(Debug, Error)]
pub struct CoreApiError {
    /// Http status code to use for the response
    pub status_code: ApiErrorStatusCode,

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

/// Http status codes available for [`CoreApiError`]
#[derive(Debug, Copy, Clone)]
pub enum ApiErrorStatusCode {
    BadRequest,
    ServerError,
    Unauthorized,
}

impl ApiErrorStatusCode {
    /// Converts the status code into `http`'s type
    pub fn to_http(&self) -> StatusCode {
        match self {
            ApiErrorStatusCode::BadRequest => StatusCode::BAD_REQUEST,
            ApiErrorStatusCode::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorStatusCode::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    /// Iterates over all available status codes
    pub fn all() -> impl Iterator<Item = Self> {
        [Self::BadRequest, Self::ServerError, Self::Unauthorized].into_iter()
    }
}

impl fmt::Display for CoreApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status_code {
            ApiErrorStatusCode::Unauthorized => write!(f, "Unauthorized")?,
            ApiErrorStatusCode::BadRequest => write!(f, "Bad Request")?,
            ApiErrorStatusCode::ServerError => write!(f, "Server Error")?,
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

impl CoreApiError {
    /// Constructs a new `CoreApiError` with [`ApiErrorStatusCode::BadRequest`]
    #[track_caller]
    pub fn bad_request(context: &'static str) -> Self {
        Self::new(ApiErrorStatusCode::BadRequest, Some(context))
    }

    /// Constructs a new `CoreApiError` with [`ApiErrorStatusCode::ServerError`]
    #[track_caller]
    pub fn server_error(context: &'static str) -> Self {
        Self::new(ApiErrorStatusCode::ServerError, Some(context))
    }

    /// Constructs a new `CoreApiError` with [`ApiErrorStatusCode::Unauthorized`]
    #[track_caller]
    pub fn unauthorized(context: &'static str) -> Self {
        Self::new(ApiErrorStatusCode::Unauthorized, Some(context))
    }

    /// Adds a source to the `CoreApiError`
    pub fn with_source(self, source: impl Error + Send + Sync + 'static) -> Self {
        self.with_boxed_source(source.into())
    }

    /// Adds a source to the `CoreApiError`
    pub fn with_boxed_source(mut self, source: Box<dyn Error + Send + Sync + 'static>) -> Self {
        self.source = Some(source);
        self
    }

    /// Adds a location to the `ApiError`
    ///
    /// Normally the location which is added automatically is enough.
    pub fn with_manual_location(mut self, location: &'static Location<'static>) -> Self {
        self.location = location;
        self
    }

    /// Creates a closure for wrapping any error into an `CoreApiError::server_error`
    ///
    /// This is just a less noisy shorthand for `|error| CoreApiError::server_error("...").with_source(error)`.
    #[track_caller]
    pub fn map_server_error<E: Error + Send + Sync + 'static>(
        context: &'static str,
    ) -> impl Fn(E) -> Self {
        let location = Location::caller();
        move |error| {
            Self::server_error(context)
                .with_source(error)
                .with_manual_location(location)
        }
    }

    /// Emit a tracing event `error!` or `debug!` describing the `CoreApiError`
    pub fn emit_tracing_event(&self) {
        let Self {
            status_code,
            context,
            location,
            source,
            #[cfg(feature = "opentelemetry")]
                trace_id: _, // The log message will hopefully be emitted in the same span
        } = &self;

        match status_code {
            ApiErrorStatusCode::Unauthorized | ApiErrorStatusCode::BadRequest => {
                debug!(
                    error.status_code = status_code.to_http().as_u16(),
                    error.status_message = status_code.to_http().as_str(),
                    error.context = context,
                    error.file = location.file(),
                    error.line = location.line(),
                    error.column = location.column(),
                    error.display = source.as_ref().map(tracing::field::display),
                    error.debug = source.as_ref().map(tracing::field::debug),
                    "Client error"
                );
            }
            ApiErrorStatusCode::ServerError => {
                error!(
                    error.status_code = status_code.to_http().as_u16(),
                    error.status_message = status_code.to_http().as_str(),
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

    /// Constructs a new `CoreApiError`
    #[track_caller]
    fn new(status_code: ApiErrorStatusCode, context: Option<&'static str>) -> Self {
        Self {
            status_code,
            context,
            location: Location::caller(),
            source: None,
            #[cfg(feature = "opentelemetry")]
            trace_id: Self::get_trace_id(),
        }
    }
}

impl IntoResponse for CoreApiError {
    fn into_response(self) -> Response {
        self.emit_tracing_event();

        let response = ApiErrorResponse {
            #[cfg(feature = "opentelemetry")]
            trace_id: self.trace_id.to_string(),
        };

        (self.status_code.to_http(), ApiJson(response)).into_response()
    }
}

impl ShouldBeResponseBody for CoreApiError {}
impl ResponseBody for CoreApiError {
    fn body(ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(mime::Mime, Option<Schema>)>)> {
        let schema = ctx.generator.generate::<ApiErrorResponse>();
        ApiErrorStatusCode::all()
            .map(|status_code| {
                (
                    status_code.to_http(),
                    Some((mime::APPLICATION_JSON, Some(schema.clone()))),
                )
            })
            .collect()
    }
}

impl<'rf, E, M> From<UpdateBuilder<'rf, E, M, rorm::crud::update::columns::Empty>>
    for CoreApiError
{
    #[track_caller]
    fn from(_value: UpdateBuilder<'rf, E, M, rorm::crud::update::columns::Empty>) -> Self {
        Self::bad_request("Nothing to update")
    }
}

trait IntoServerError: Into<Box<dyn Error + Send + Sync + 'static>> {}
impl<E: IntoServerError> From<E> for CoreApiError {
    #[track_caller]
    fn from(value: E) -> Self {
        Self {
            status_code: ApiErrorStatusCode::ServerError,
            context: None,
            location: Location::caller(),
            source: Some(value.into()),
            #[cfg(feature = "opentelemetry")]
            trace_id: Self::get_trace_id(),
        }
    }
}
impl IntoServerError for rorm::Error {}
impl IntoServerError for tower_sessions::session::Error {}
impl IntoServerError for anyhow::Error {}
