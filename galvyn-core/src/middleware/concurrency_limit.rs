//! Middleware which limits the number of concurrent requests.

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use axum::body::HttpBody;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use bytes::Bytes;
use http_body::Frame;
use pin_project_lite::pin_project;
use tokio::sync::Semaphore;
use tokio::sync::TryAcquireError;

use crate::middleware::AxumService;
use crate::middleware::GalvynMiddleware;

/// Middleware which limits the number of concurrent requests.
#[derive(Clone)]
pub struct ConcurrencyLimitMiddleware<F = DefaultHandleConcurrencyLimit> {
    inner: Arc<(Semaphore, ConcurrencyLimitParams<F>)>,
}

impl ConcurrencyLimitMiddleware<DefaultHandleConcurrencyLimit> {
    /// Constructs a new `ConcurrencyLimit` defaulting everything but the `max`
    pub fn default(max: usize) -> Self {
        Self::new(ConcurrencyLimitParams {
            max,
            count_response_bodies: false,
            then: DefaultHandleConcurrencyLimit,
        })
    }
}

impl<F> ConcurrencyLimitMiddleware<F> {
    /// Constructs a new `ConcurrencyLimit`
    pub fn new(params: ConcurrencyLimitParams<F>) -> Self {
        Self {
            inner: Arc::new((Semaphore::new(params.max), params)),
        }
    }
}

/// Parameters to [`ConcurrencyLimitMiddleware::new`]
pub struct ConcurrencyLimitParams<F> {
    /// Maximum number of concurrent requests
    pub max: usize,

    /// Do response bodies still count towards the concurrency limit?
    ///
    /// The default is `false` because it is easier to reason about
    /// (from the perspective of a server author) and has less overhead.
    ///
    /// This flag makes no difference unless you stream response bodies.
    pub count_response_bodies: bool,

    /// Callback used to handle limited requests.
    pub then: F,
}

impl<F: HandleConcurrencyLimit> GalvynMiddleware for ConcurrencyLimitMiddleware<F> {
    async fn call<S: AxumService>(
        self,
        mut inner: S,
        request: Request,
    ) -> Result<Response, Infallible> {
        let (semaphore, params) = self.inner.as_ref();

        let permit = match semaphore.try_acquire() {
            Ok(x) => x,
            Err(TryAcquireError::NoPermits) => {
                match params.then.clone().handle_concurrency_limit(&request) {
                    ControlFlow::Break(response) => return Ok(response),
                    ControlFlow::Continue(()) => match semaphore.acquire().await {
                        Ok(x) => x,
                        Err(_closed) => unreachable!(),
                    },
                }
            }
            Err(TryAcquireError::Closed) => unreachable!(),
        };

        let Ok(response) = inner.call(request).await;
        let mut response = response.into_response();

        if params.count_response_bodies {
            permit.forget();
            response = response.map(move |body| {
                axum::body::Body::new(BodyWithDrop {
                    body,
                    drop: Some(move || {
                        self.inner.0.add_permits(1);
                    }),
                })
            });
        }

        Ok(response)
    }
}

/// Closure used by [`ConcurrencyLimitMiddleware`] to handle incoming requests while the concurrency limit has been reached.
///
/// This trait will be auto-implemented for closures of the appropriate bounds.
pub trait HandleConcurrencyLimit: Clone + Send + Sync + 'static {
    /// Handles incoming requests while the concurrency limit has been reached
    ///
    /// Returns `Break(response)` to reject the request
    /// or `Continue(())` to wait until other requests finished.
    /// (This is subject to timeouts others may define)
    fn handle_concurrency_limit(self, request: &Request) -> ControlFlow<Response>;
}
impl<F> HandleConcurrencyLimit for F
where
    F: Clone + Send + Sync + 'static,
    F: FnOnce(&Request) -> ControlFlow<Response>,
{
    fn handle_concurrency_limit(self, request: &Request) -> ControlFlow<Response> {
        self(request)
    }
}

/// Default implementation for [`ConcurrencyLimitMiddleware`]
///
/// It will return a basic "429"
#[derive(Copy, Clone, Debug)]
pub struct DefaultHandleConcurrencyLimit;
impl HandleConcurrencyLimit for crate::middleware::catch_unwind::DefaultHandlePanic {
    fn handle_concurrency_limit(self, _request: &Request) -> ControlFlow<Response> {
        ControlFlow::Break(
            (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many concurrent requests",
            )
                .into_response(),
        )
    }
}

pin_project! {
    struct BodyWithDrop<F: FnOnce()> {
        #[pin]
        body: axum::body::Body,
        drop: Option<F>,
    }

    impl<F: FnOnce()> PinnedDrop for BodyWithDrop<F> {
        fn drop(this: Pin<&mut Self>) {
            if let Some(x) = this.project().drop.take() {
                x();
            }
        }
    }
}
impl<F: FnOnce()> HttpBody for BodyWithDrop<F> {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().body.poll_frame(cx)
    }
}
