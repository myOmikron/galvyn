//! Middleware which catches stack unwinding cased by a panic
//! and converts it into a `500` response and a logged error.

use std::any::Any;
use std::convert::Infallible;
use std::future::poll_fn;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::pin;
use std::task::Poll;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::response::Response;

use crate::middleware::AxumService;
use crate::middleware::GalvynMiddleware;
use crate::stuff::api_error::core::CoreApiError;

/// Middleware which catches stack unwinding cased by a panic
/// and converts it into a `500` response and a logged error.
#[derive(Copy, Clone, Debug)]
pub struct CatchUnwindMiddleware<F> {
    /// Callback used to produce the `500` response
    pub then: F,
}

impl Default for CatchUnwindMiddleware<DefaultHandlePanic> {
    fn default() -> Self {
        Self {
            then: DefaultHandlePanic,
        }
    }
}

impl<F: HandlePanic> GalvynMiddleware for CatchUnwindMiddleware<F> {
    async fn call<S: AxumService>(
        self,
        mut inner: S,
        request: Request,
    ) -> Result<Response, Infallible> {
        let mut inner = pin!(inner.call(request));
        Ok(poll_fn(
            |cx| match catch_unwind(AssertUnwindSafe(|| inner.as_mut().poll(cx))) {
                Ok(Poll::Pending) => Poll::Pending,
                Ok(Poll::Ready(res)) => Poll::Ready(res.into_response()),
                Err(payload) => Poll::Ready(self.then.clone().handle_panic(payload)),
            },
        )
        .await)
    }
}

/// Closure used by [`CatchUnwindMiddleware`] to produce the response for a caught panic
///
/// This trait will be auto-implemented for closures of the appropriate bounds.
pub trait HandlePanic: Clone + Send + Sync + 'static {
    /// Produces the response returned by [`CatchUnwindMiddleware`] for a caught panic
    fn handle_panic(self, payload: Box<dyn Any + Send + 'static>) -> Response;
}
impl<F> HandlePanic for F
where
    F: Clone + Send + Sync + 'static,
    F: FnOnce(Box<dyn Any + Send + 'static>) -> Response,
{
    fn handle_panic(self, payload: Box<dyn Any + Send + 'static>) -> Response {
        self(payload)
    }
}

/// Default implementation for [`CatchUnwindMiddleware`]
///
/// It will return a basic [`CoreApiError`]
#[derive(Copy, Clone, Debug)]
pub struct DefaultHandlePanic;
impl HandlePanic for DefaultHandlePanic {
    fn handle_panic(self, _payload: Box<dyn Any + Send + 'static>) -> Response {
        CoreApiError::server_error("Caught panic in handler").into_response()
    }
}
