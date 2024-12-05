//! Set of global managers and handles

use std::ops::Deref;
use std::sync::OnceLock;

use rorm::Database;
use webauthn_rs::prelude::AttestationCaList;
use webauthn_rs::Webauthn;

use crate::global::ws::GlobalWs;

pub mod ws;

/// Set of global managers and handles
pub static GLOBAL: GlobalOnceCell<GlobalEntities> = GlobalOnceCell::new();

/// Set of global managers and handles
pub struct GlobalEntities {
    /// The database
    pub db: Database,

    /// The global websocket instance
    pub ws: GlobalWs,

    /// Global WebAuthn state
    pub webauthn: Webauthn,

    /// List of attestation cas accepted when registering new webauthn keys with login privileges.
    pub webauthn_attestation_ca_list: AttestationCaList,

    /// The url this server is reachable under
    ///
    /// Used for generating links which should point back to mannequin
    pub origin: String,
}

/// Simple [`OnceLock`] which panics in case of error.
pub struct GlobalOnceCell<T>(OnceLock<T>);
impl<T> GlobalOnceCell<T> {
    /// Creates a new empty cell
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }

    /// Check if the OnceLock is already initialized
    pub fn is_initialized(&self) -> bool {
        self.0.get().is_some()
    }

    /// Initialise the cell
    ///
    /// ## Panics
    /// If called twice
    pub fn init(&self, value: T) {
        self.0
            .set(value)
            .ok()
            .expect("`GlobalLock.init` has been called twice")
    }
}
impl<T> Deref for GlobalOnceCell<T> {
    type Target = T;

    /// Retrieved the initialised value
    ///
    /// ## Panics
    /// If called before [`GlobalOnceCell::init`]
    fn deref(&self) -> &Self::Target {
        #[allow(clippy::expect_used)]
        self.0
            .get()
            .expect("`GlobalLock.init` has not been called yet. Please open an issues.")
    }
}
