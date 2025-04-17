use galvyn_core::re_exports::uuid::Uuid;
use std::fmt;

/// Setup for the [`OauthProviderModule`](crate::OauthProviderModule)
#[derive(Debug)]
pub struct OauthProviderSetup {
    pub frontend_redirect: Box<dyn FrontendRedirect>,
}

impl Default for OauthProviderSetup {
    fn default() -> Self {
        Self {
            frontend_redirect: Box::new(DefaultFrontendRedirect),
        }
    }
}

pub trait FrontendRedirect: fmt::Debug + Send + Sync + 'static {
    fn redirect_uri(&self, request_uuid: Uuid) -> String;
}
impl<F> FrontendRedirect for F
where
    F: Fn(Uuid) -> String,
    F: fmt::Debug + Send + Sync + 'static,
{
    fn redirect_uri(&self, request_uuid: Uuid) -> String {
        self(request_uuid)
    }
}

/// The [`OauthProviderSetup`]'s default `frontend_redirect`
#[derive(Debug)]
pub struct DefaultFrontendRedirect;
impl FrontendRedirect for DefaultFrontendRedirect {
    fn redirect_uri(&self, _request_uuid: Uuid) -> String {
        unimplemented!("We could ship a simple static html for that™")
    }
}
