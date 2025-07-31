//! A middleware wraps a group of handler to alter their behaviour.
//!
//! The contained traits are essentially simpler versions of [`tower::Layer`].

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::task::Context;
use std::task::Poll;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::response::Response;
use futures_lite::future::Boxed;
use tower::Layer;
use tower::Service;

/// A middleware wraps a group of handler to alter their behaviour.
///
/// This trait is an even simpler version of [`GalvynMiddleware`].
/// Its methods run before and after the actual endpoint handler,
/// but can't alter the actual execution of the handler.
///
/// If you can't express your logic with this trait,
/// feel free to use `GalvynMiddleware` or even [`tower::Layer`] instead.
pub trait SimpleGalvynMiddleware: Clone + Send + Sync + 'static {
    /// Pre-process a request and might choose to return a response without running the handler.
    fn pre_handler(
        &mut self,
        request: Request,
    ) -> impl Future<Output = ControlFlow<Response, Request>> + Send {
        async move { ControlFlow::Continue(request) }
    }

    /// Post-process the handler's response
    fn post_handler(&mut self, response: Response) -> impl Future<Output = Response> + Send {
        async move { response }
    }
}

/// A middleware wraps a group of handler to alter their behaviour.
///
/// This trait is a simplified version of [`tower::Layer`]s.
/// It is specialized for the usage with galvyn (`axum` under the hood)
/// but can be converted into a `Layer` implementation through the [`MiddlewareLayer`] adapter.
///
/// You can try [`SimpleGalvynMiddleware`] for the simplest use cases.
///
/// # Restrictions compared to `Layer`
///
/// - only supports [`AxumService`]s (i.e., [`tower::Service`] intended for `axum`)
/// - does not support back-pressure (i.e., `Layer::poll_ready`)
/// - clones `Self` on every request
///
/// Those restrictions seem reasonable for any layer written for axum by an application author.
///
/// If they are too limiting, feel free to user `Layer` instead.
pub trait GalvynMiddleware: Clone + Send + Sync + 'static {
    /// Processes a request
    fn call<S: AxumService>(
        self,
        inner: S,
        request: Request,
    ) -> impl Future<Output = Result<Response, Infallible>> + Send + 'static;

    /// Wraps the middleware in an adapter to implement [`tower::Layer`]
    fn into_layer(self) -> MiddlewareLayer<Self>
    where
        Self: Sized,
    {
        MiddlewareLayer(self)
    }
}

impl<T: SimpleGalvynMiddleware> GalvynMiddleware for T {
    async fn call<S: AxumService>(
        mut self,
        mut inner: S,
        request: Request,
    ) -> Result<Response, Infallible> {
        Ok(match self.pre_handler(request).await {
            ControlFlow::Continue(request) => {
                let response = inner.call(request).await.into_response();
                self.post_handler(response).await
            }
            ControlFlow::Break(response) => response,
        })
    }
}

/// Adapter to implement [`tower::Layer`] for [`GalvynMiddleware`]s
#[derive(Copy, Clone, Debug)]
pub struct MiddlewareLayer<M>(pub M);

impl<M, S> Layer<S> for MiddlewareLayer<M>
where
    M: GalvynMiddleware,
{
    type Service = MiddlewareService<M, S>;
    fn layer(&self, inner: S) -> Self::Service {
        MiddlewareService {
            inner,
            middleware: self.0.clone(),
        }
    }
}

/// [`tower::Service`] produce by [`MiddlewareLayer`]
#[derive(Copy, Clone, Debug)]
pub struct MiddlewareService<M, S> {
    inner: S,
    middleware: M,
}

impl<M, S> Service<Request> for MiddlewareService<M, S>
where
    M: GalvynMiddleware,
    S: AxumService,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Boxed<Result<Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let not_ready_inner = self.inner.clone();
        let ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);
        let middleware = self.middleware.clone();

        Box::pin(middleware.call(ready_inner, request))
    }
}

/// Trait alias for [`tower::Service`] constraint to be used by axum
pub trait AxumService:
    Service<Request, Error = Infallible, Response: IntoResponse, Future: Send + 'static>
    + Clone
    + Send
    + 'static
{
}
impl<T> AxumService for T where
    T: Service<Request, Error = Infallible, Response: IntoResponse, Future: Send + 'static>
        + Clone
        + Send
        + 'static
{
}
