use time::Duration;
use tower_cookies::cookie::SameSite;

pub use self::service::SessionManager;
pub use self::service::SessionManagerLayer;
//
#[doc(inline)]
pub use self::session::{Expiry, Session};
use crate::session::session::Id;
use crate::session::store::rorm::RormStore;

pub mod service;
pub mod session;
pub mod store;

pub fn layer() -> SessionManagerLayer {
    SessionManagerLayer::new(RormStore::default())
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)))
        .with_same_site(SameSite::Lax)
        .with_always_save(true)
}
