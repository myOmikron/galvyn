//! A middleware that provides [`Session`] as a request extension.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use axum::http;
use axum::http::Request;
use axum::http::Response;
use tower::Layer;
use tower::Service;
use tower_cookies::Cookie;
use tower_cookies::CookieManager;
use tower_cookies::Cookies;
use tracing::Instrument;

use crate::session::Session;
use crate::session::config::SessionConfig;
use crate::session::store::SessionStore;
use crate::session::store::StoreAndConfig;
use crate::session::{self};

/// A middleware that provides [`Session`] as a request extension.
#[derive(Debug, Clone)]
pub struct SessionManager<S> {
    inner: S,
    store: Arc<StoreAndConfig>,
}

impl<S> SessionManager<S> {
    /// Create a new [`SessionManager`].
    pub fn new(inner: S, store: impl SessionStore, config: SessionConfig) -> Self {
        Self {
            inner,
            store: Arc::new(StoreAndConfig { config, store }),
        }
    }
}

impl<ReqBody, ResBody, S> Service<Request<ReqBody>> for SessionManager<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let span = tracing::info_span!("call");

        let store = self.store.clone();

        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(
            async move {
                let Some(cookies) = req.extensions().get::<Cookies>().cloned() else {
                    // In practice this should never happen because we wrap `CookieManager`
                    // directly.
                    tracing::error!("missing cookies request extension");
                    return Ok(Response::default());
                };

                let session_cookie = cookies.get(&store.config.name).map(Cookie::into_owned);
                let session_id = session_cookie.as_ref().and_then(|cookie| {
                    cookie
                        .value()
                        .parse::<session::Id>()
                        .map_err(|err| {
                            tracing::warn!(
                                err = %err,
                                "possibly suspicious activity: malformed session id"
                            )
                        })
                        .ok()
                });

                let session = Session::new(session_id, store.clone());

                req.extensions_mut().insert(session.clone());

                let res = inner.call(req).await?;

                let modified = session.is_modified();
                let empty = session.is_empty().await;

                tracing::trace!(
                    modified = modified,
                    empty = empty,
                    always_save = store.config.always_save,
                    "session response state",
                );

                match session_cookie {
                    Some(mut cookie) if empty => {
                        tracing::debug!("removing session cookie");

                        // Path and domain must be manually set to ensure a proper removal cookie is
                        // constructed.
                        //
                        // See: https://docs.rs/cookie/latest/cookie/struct.CookieJar.html#method.remove
                        cookie.set_path(store.config.path.clone());
                        if let Some(domain) = store.config.domain.clone() {
                            cookie.set_domain(domain);
                        }

                        // Preserve the configured Secure and HttpOnly attributes. Otherwise
                        // `__Host-`/`__Secure-`-prefixed cookies without Secure may be rejected.
                        cookie.set_secure(store.config.secure);
                        cookie.set_http_only(store.config.http_only);

                        cookies.remove(cookie);
                    }

                    _ if (modified || store.config.always_save)
                        && !empty
                        && !res.status().is_server_error() =>
                    {
                        tracing::debug!("saving session");
                        if let Err(err) = session.save().await {
                            tracing::error!(err = %err, "failed to save session");

                            let mut res = Response::default();
                            *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                            return Ok(res);
                        }

                        let Some(session_id) = session.id() else {
                            tracing::error!("missing session id");

                            let mut res = Response::default();
                            *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                            return Ok(res);
                        };

                        let expiry = session.expiry();
                        let session_cookie = store.config.build_cookie(session_id, expiry);

                        tracing::debug!("adding session cookie");
                        cookies.add(session_cookie);
                    }

                    _ => (),
                };

                Ok(res)
            }
            .instrument(span),
        )
    }
}

/// A layer for providing [`Session`] as a request extension.
#[derive(Debug, Clone)]
pub struct SessionManagerLayer {
    store: Arc<StoreAndConfig>,
}

impl SessionManagerLayer {
    /// Create a new [`SessionManagerLayer`] with the provided session store
    /// and default cookie configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tower_sessions::{MemoryStore, SessionManagerLayer};
    ///
    /// let session_store = MemoryStore::default();
    /// let session_service = SessionManagerLayer::new(session_store);
    /// ```
    pub fn new(store: impl SessionStore, config: SessionConfig) -> Self {
        Self {
            store: Arc::new(StoreAndConfig { config, store }),
        }
    }
}

impl<S> Layer<S> for SessionManagerLayer {
    type Service = CookieManager<SessionManager<S>>;

    fn layer(&self, inner: S) -> Self::Service {
        let session_manager = SessionManager {
            inner,
            store: self.store.clone(),
        };

        CookieManager::new(session_manager)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::anyhow;
    use axum::body::Body;
    use tower::ServiceBuilder;
    use tower::ServiceExt;

    use super::*;
    use crate::session::session::Record;
    use crate::session::store::memory::MemoryStore;

    async fn handler(req: Request<Body>) -> anyhow::Result<Response<Body>> {
        let session = req
            .extensions()
            .get::<Session>()
            .ok_or(anyhow!("Missing session"))?;

        session.insert("foo", 42).await?;

        Ok(Response::new(Body::empty()))
    }

    async fn noop_handler(_: Request<Body>) -> anyhow::Result<Response<Body>> {
        Ok(Response::new(Body::empty()))
    }

    async fn flush_handler(req: Request<Body>) -> anyhow::Result<Response<Body>> {
        let session = req
            .extensions()
            .get::<Session>()
            .ok_or(anyhow!("Missing session"))?;

        session.flush().await?;

        Ok(Response::new(Body::empty()))
    }

    #[tokio::test]
    async fn basic_service_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.clone().oneshot(req).await?;

        let session = res.headers().get(http::header::SET_COOKIE);
        assert!(session.is_some());

        let req = Request::builder()
            .header(http::header::COOKIE, session.unwrap())
            .body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_none());

        Ok(())
    }

    #[tokio::test]
    async fn bogus_cookie_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.clone().oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_some());

        let req = Request::builder()
            .header(http::header::COOKIE, "id=bogus")
            .body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_some());

        Ok(())
    }

    #[tokio::test]
    async fn no_set_cookie_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(noop_handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_none());

        Ok(())
    }

    #[tokio::test]
    async fn name_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_name("my.sid");
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.starts_with("my.sid=")));

        Ok(())
    }

    #[tokio::test]
    async fn http_only_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("HttpOnly")));

        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_http_only(false);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| !s.contains("HttpOnly")));

        Ok(())
    }

    #[tokio::test]
    async fn same_site_strict_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer =
            SessionManagerLayer::new(session_store).with_same_site(SameSite::Strict);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("SameSite=Strict")));

        Ok(())
    }

    #[tokio::test]
    async fn same_site_lax_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_same_site(SameSite::Lax);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("SameSite=Lax")));

        Ok(())
    }

    #[tokio::test]
    async fn same_site_none_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_same_site(SameSite::None);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("SameSite=None")));

        Ok(())
    }

    #[tokio::test]
    async fn expiry_on_session_end_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer =
            SessionManagerLayer::new(session_store).with_expiry(Expiry::OnSessionEnd);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| !s.contains("Max-Age")));

        Ok(())
    }

    #[tokio::test]
    async fn expiry_on_inactivity_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let inactivity_duration = time::Duration::hours(2);
        let session_layer = SessionManagerLayer::new(session_store)
            .with_expiry(Expiry::OnInactivity(inactivity_duration));
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        let expected_max_age = inactivity_duration.whole_seconds();
        assert!(cookie_has_expected_max_age(&res, expected_max_age));

        Ok(())
    }

    #[tokio::test]
    async fn expiry_at_date_time_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let expiry_time = time::OffsetDateTime::now_utc() + time::Duration::weeks(1);
        let session_layer =
            SessionManagerLayer::new(session_store).with_expiry(Expiry::AtDateTime(expiry_time));
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        let expected_max_age = (expiry_time - time::OffsetDateTime::now_utc()).whole_seconds();
        assert!(cookie_has_expected_max_age(&res, expected_max_age));

        Ok(())
    }

    #[tokio::test]
    async fn expiry_on_session_end_always_save_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store.clone())
            .with_expiry(Expiry::OnSessionEnd)
            .with_always_save(true);
        let mut svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req1 = Request::builder().body(Body::empty())?;
        let res1 = svc.call(req1).await?;
        let sid1 = get_session_id(&res1);
        let rec1 = get_record(&session_store, &sid1).await;
        let req2 = Request::builder()
            .header(http::header::COOKIE, format!("id={}", sid1))
            .body(Body::empty())?;
        let res2 = svc.call(req2).await?;
        let sid2 = get_session_id(&res2);
        let rec2 = get_record(&session_store, &sid2).await;

        assert!(cookie_value_matches(&res2, |s| !s.contains("Max-Age")));
        assert!(sid1 == sid2);
        assert!(rec1.expiry_date < rec2.expiry_date);

        Ok(())
    }

    #[tokio::test]
    async fn expiry_on_inactivity_always_save_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let inactivity_duration = time::Duration::hours(2);
        let session_layer = SessionManagerLayer::new(session_store.clone())
            .with_expiry(Expiry::OnInactivity(inactivity_duration))
            .with_always_save(true);
        let mut svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req1 = Request::builder().body(Body::empty())?;
        let res1 = svc.call(req1).await?;
        let sid1 = get_session_id(&res1);
        let rec1 = get_record(&session_store, &sid1).await;
        let req2 = Request::builder()
            .header(http::header::COOKIE, format!("id={}", sid1))
            .body(Body::empty())?;
        let res2 = svc.call(req2).await?;
        let sid2 = get_session_id(&res2);
        let rec2 = get_record(&session_store, &sid2).await;

        let expected_max_age = inactivity_duration.whole_seconds();
        assert!(cookie_has_expected_max_age(&res2, expected_max_age));
        assert!(sid1 == sid2);
        assert!(rec1.expiry_date < rec2.expiry_date);

        Ok(())
    }

    #[tokio::test]
    async fn expiry_at_date_time_always_save_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let expiry_time = time::OffsetDateTime::now_utc() + time::Duration::weeks(1);
        let session_layer = SessionManagerLayer::new(session_store.clone())
            .with_expiry(Expiry::AtDateTime(expiry_time))
            .with_always_save(true);
        let mut svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req1 = Request::builder().body(Body::empty())?;
        let res1 = svc.call(req1).await?;
        let sid1 = get_session_id(&res1);
        let rec1 = get_record(&session_store, &sid1).await;
        let req2 = Request::builder()
            .header(http::header::COOKIE, format!("id={}", sid1))
            .body(Body::empty())?;
        let res2 = svc.call(req2).await?;
        let sid2 = get_session_id(&res2);
        let rec2 = get_record(&session_store, &sid2).await;

        let expected_max_age = (expiry_time - time::OffsetDateTime::now_utc()).whole_seconds();
        assert!(cookie_has_expected_max_age(&res2, expected_max_age));
        assert!(sid1 == sid2);
        assert!(rec1.expiry_date == rec2.expiry_date);

        Ok(())
    }

    #[tokio::test]
    async fn secure_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_secure(true);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("Secure")));

        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| !s.contains("Secure")));

        Ok(())
    }

    #[tokio::test]
    async fn secure_removal_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store.clone()).with_secure(true);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        let session_cookie = response_cookie(&res)?;

        let flush_layer = SessionManagerLayer::new(session_store).with_secure(true);
        let flush_svc = ServiceBuilder::new()
            .layer(flush_layer)
            .service_fn(flush_handler);

        let req = Request::builder()
            .header(http::header::COOKIE, cookie_request_value(&session_cookie))
            .body(Body::empty())?;
        let res = flush_svc.oneshot(req).await?;

        let removal_cookie = response_cookie(&res)?;
        assert_eq!(removal_cookie.secure(), Some(true));

        Ok(())
    }

    #[tokio::test]
    async fn http_only_removal_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store.clone()).with_http_only(true);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;
        let session_cookie = response_cookie(&res)?;

        let flush_layer = SessionManagerLayer::new(session_store).with_http_only(true);
        let flush_svc = ServiceBuilder::new()
            .layer(flush_layer)
            .service_fn(flush_handler);

        let req = Request::builder()
            .header(http::header::COOKIE, cookie_request_value(&session_cookie))
            .body(Body::empty())?;
        let res = flush_svc.oneshot(req).await?;

        let removal_cookie = response_cookie(&res)?;
        assert_eq!(removal_cookie.http_only(), Some(true));

        Ok(())
    }

    #[tokio::test]
    async fn http_only_disabled_removal_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store.clone()).with_http_only(false);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;
        let session_cookie = response_cookie(&res)?;

        let flush_layer = SessionManagerLayer::new(session_store).with_http_only(false);
        let flush_svc = ServiceBuilder::new()
            .layer(flush_layer)
            .service_fn(flush_handler);

        let req = Request::builder()
            .header(http::header::COOKIE, cookie_request_value(&session_cookie))
            .body(Body::empty())?;
        let res = flush_svc.oneshot(req).await?;

        let removal_cookie = response_cookie(&res)?;
        assert_eq!(removal_cookie.http_only(), None);

        Ok(())
    }

    #[tokio::test]
    async fn insecure_removal_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store.clone()).with_secure(false);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        let session_cookie = response_cookie(&res)?;

        let flush_layer = SessionManagerLayer::new(session_store).with_secure(false);
        let flush_svc = ServiceBuilder::new()
            .layer(flush_layer)
            .service_fn(flush_handler);

        let req = Request::builder()
            .header(http::header::COOKIE, cookie_request_value(&session_cookie))
            .body(Body::empty())?;
        let res = flush_svc.oneshot(req).await?;

        let removal_cookie = response_cookie(&res)?;

        assert_eq!(removal_cookie.secure(), None);

        Ok(())
    }

    #[tokio::test]
    async fn path_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_path("/foo/bar");
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("Path=/foo/bar")));

        Ok(())
    }

    #[tokio::test]
    async fn domain_test() -> anyhow::Result<()> {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_domain("example.com");
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(cookie_value_matches(&res, |s| s.contains("Domain=example.com")));

        Ok(())
    }

    #[cfg(feature = "signed")]
    #[tokio::test]
    async fn signed_test() -> anyhow::Result<()> {
        let key = Key::generate();
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_signed(key);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_some());

        Ok(())
    }

    #[cfg(feature = "private")]
    #[tokio::test]
    async fn private_test() -> anyhow::Result<()> {
        let key = Key::generate();
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store).with_private(key);
        let svc = ServiceBuilder::new()
            .layer(session_layer)
            .service_fn(handler);

        let req = Request::builder().body(Body::empty())?;
        let res = svc.oneshot(req).await?;

        assert!(res.headers().get(http::header::SET_COOKIE).is_some());

        Ok(())
    }

    fn cookie_value_matches<F>(res: &Response<Body>, matcher: F) -> bool
    where
        F: FnOnce(&str) -> bool,
    {
        res.headers()
            .get(http::header::SET_COOKIE)
            .is_some_and(|set_cookie| set_cookie.to_str().is_ok_and(matcher))
    }

    fn response_cookie(res: &Response<Body>) -> anyhow::Result<Cookie<'static>> {
        let value = res
            .headers()
            .get(http::header::SET_COOKIE)
            .ok_or(anyhow!("Missing Set-Cookie header"))?;

        Ok(Cookie::parse(value.to_str()?.to_owned())?.into_owned())
    }

    fn cookie_request_value(cookie: &Cookie<'_>) -> String {
        format!("{}={}", cookie.name(), cookie.value())
    }

    fn cookie_has_expected_max_age(res: &Response<Body>, expected_value: i64) -> bool {
        res.headers()
            .get(http::header::SET_COOKIE)
            .is_some_and(|set_cookie| {
                set_cookie.to_str().is_ok_and(|s| {
                    let max_age_value = s
                        .split("Max-Age=")
                        .nth(1)
                        .unwrap_or_default()
                        .split(';')
                        .next()
                        .unwrap_or_default()
                        .parse::<i64>()
                        .unwrap_or_default();
                    (max_age_value - expected_value).abs() <= 1
                })
            })
    }

    fn get_session_id(res: &Response<Body>) -> String {
        res.headers()
            .get(http::header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split("id=")
            .nth(1)
            .unwrap()
            .split(";")
            .next()
            .unwrap()
            .to_string()
    }

    async fn get_record(store: &impl SessionStore, id: &str) -> Record {
        store
            .load(&Id::from_str(id).unwrap())
            .await
            .unwrap()
            .unwrap()
    }
}
