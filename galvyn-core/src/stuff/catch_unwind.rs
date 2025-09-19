//! Middleware which catches stack unwinding cased by a panic
//! and converts it into a `500` response and a logged error.

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
pub struct CatchUnwindLayer;
impl GalvynMiddleware for CatchUnwindLayer {
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
                Err(_payload) => Poll::Ready(
                    CoreApiError::server_error("Caught panic in handler").into_response(),
                ),
            },
        )
        .await)
    }
}
