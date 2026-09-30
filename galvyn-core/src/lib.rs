use std::convert::Infallible;

use axum::extract::Request;
use axum::response::IntoResponse;
use tower::Service;

pub mod re_exports {
    pub use axum;
}

/// Trait alias for [`tower::Service`] constraint to be used by axum
pub trait AxumService:
    Service<Request, Error = Infallible, Response: IntoResponse, Future: Send + 'static>
{
}
impl<T> AxumService for T where
    T: Service<Request, Error = Infallible, Response: IntoResponse, Future: Send + 'static>
{
}
