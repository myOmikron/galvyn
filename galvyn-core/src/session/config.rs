use std::borrow::Cow;

use time::OffsetDateTime;
use tower_cookies::Cookie;
use tower_cookies::cookie::SameSite;

use crate::session::Expiry;
use crate::session::session::Id;

/// Config how the session layer and session cookie should behave
#[derive(Debug, Clone)]
#[cfg_attr(doc, non_exhaustive)]
pub struct SessionConfig {
    /// Configures the name of the cookie used for the session.
    /// The default value is `"id"`.
    pub name: Cow<'static, str>,

    /// Configures the `"HttpOnly"` attribute of the cookie used for the
    /// session.
    ///
    /// # ⚠️ **Warning: Cross-site scripting risk**
    ///
    /// Applications should generally **not** override the default value of
    /// `true`. If you do, you are exposing your application to increased risk
    /// of cookie theft via techniques like cross-site scripting.
    pub http_only: bool,

    /// Configures the `"SameSite"` attribute of the cookie used for the
    /// session.
    /// The default value is [`SameSite::Strict`].
    pub same_site: SameSite,

    /// Configures the `"Max-Age"` attribute of the cookie used for the session.
    /// The default value is `None`.
    pub expiry: Option<Expiry>,

    /// Configures the `"Secure"` attribute of the cookie used for the session.
    /// The default value is `true`.
    pub secure: bool,

    /// Configures the `"Path"` attribute of the cookie used for the session.
    /// The default value is `"/"`.
    pub path: Cow<'static, str>,

    /// Configures the `"Domain"` attribute of the cookie used for the session.
    /// The default value is `None`.
    pub domain: Option<Cow<'static, str>>,

    /// Configures whether unmodified session should be saved on read or not.
    /// When the value is `true`, the session will be saved even if it was not
    /// changed.
    ///
    /// This is useful when you want to reset [`Session`] expiration time
    /// on any valid request at the cost of higher [`SessionStore`] write
    /// activity and transmitting `set-cookie` header with each response.
    ///
    /// It makes sense to use this setting with relative session expiration
    /// values, such as `Expiry::OnInactivity(Duration)`. This setting will
    /// _not_ cause session id to be cycled on save.
    ///
    /// The default value is `false`.
    pub always_save: bool,

    // Do not depend on this struct being exhaustive!
    #[doc(hidden)]
    pub you_are_making_a_mistake_this_struct_is_not_exhaustive: (),
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            name: "id".into(), /* See: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-name-fingerprinting */
            http_only: true,
            same_site: SameSite::Strict,
            expiry: None, // TODO: Is `Max-Age: "Session"` the right default?
            secure: true,
            path: "/".into(),
            domain: None,
            always_save: false,

            you_are_making_a_mistake_this_struct_is_not_exhaustive: (),
        }
    }
}

impl SessionConfig {
    pub(crate) fn build_cookie(&self, session_id: Id, expiry: Option<Expiry>) -> Cookie<'static> {
        let mut cookie_builder = Cookie::build((self.name.clone(), session_id.to_string()))
            .http_only(self.http_only)
            .same_site(self.same_site)
            .secure(self.secure)
            .path(self.path.clone());

        cookie_builder = match expiry {
            Some(Expiry::OnInactivity(duration)) => cookie_builder.max_age(duration),
            Some(Expiry::AtDateTime(datetime)) => {
                cookie_builder.max_age(datetime - OffsetDateTime::now_utc())
            }
            Some(Expiry::OnSessionEnd) | None => cookie_builder,
        };

        if let Some(domain) = self.domain.clone() {
            cookie_builder = cookie_builder.domain(domain);
        }

        cookie_builder.build()
    }
}
