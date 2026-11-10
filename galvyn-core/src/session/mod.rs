use time::Duration;
use tower_cookies::cookie::SameSite;

use self::service::SessionManagerLayer;
#[doc(inline)]
pub use self::session::Expiry;
#[doc(inline)]
pub use self::session::Session;
use crate::session::config::SessionConfig;
use crate::session::session::Id;
use crate::session::store::rorm::RormStore;

pub mod config;
pub mod service;
pub mod session;
pub mod store;

pub fn layer() -> SessionManagerLayer {
    SessionManagerLayer::new(
        RormStore::default(),
        SessionConfig {
            expiry: Some(Expiry::OnInactivity(Duration::hours(24))),
            same_site: SameSite::Lax,
            always_save: true,
            ..Default::default()
        },
    )
}
