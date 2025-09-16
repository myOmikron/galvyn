//! Middleware which catches stack unwinding cased by a panic
//! and converts it into a `500` response and a logged error.

use std::convert::Infallible;
use std::panic::AssertUnwindSafe;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::response::Response;
use futures::FutureExt;

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
        match AssertUnwindSafe(inner.call(request)).catch_unwind().await {
            Ok(response) => Ok(response.into_response()),
            Err(_payload) => {
                Ok(CoreApiError::server_error("Caught panic in handler").into_response())
            }
        }
    }
}
