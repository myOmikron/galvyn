use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::PoisonError;

use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PreInitError;
use galvyn_core::re_exports::rorm::Database;
use galvyn_core::re_exports::uuid::Uuid;

use crate::OauthProviderSetup;

pub struct OauthProviderModule {
    pub(crate) db: Database,

    pub(crate) setup: OauthProviderSetup,

    /// Waiting for user interaction i.e. `/accept` or `/deny`
    ///
    /// Uses a `uuid` as key which is presented to the user's agent
    open_requests: Mutex<HashMap<Uuid, OauthRequest>>,

    /// Waiting for server interaction i.e. `/token`
    ///
    /// Uses `code` as key which is passed through the user's agent to the client
    accepted_requests: Mutex<HashMap<Uuid, OauthRequest>>,
}

/// Information about an ongoing oauth request
#[derive(Debug, Clone)]
pub struct OauthRequest {
    /// The requesting [`GalvynOauthClient`](crate::models::GalvynOauthClient)'s uuid
    pub client_uuid: Uuid,

    /// State provided by client in `/auth`
    pub state: String,

    /// Scope requested by client
    pub scope: (),

    /// pkce's `code_challenge` with method `S256`
    pub code_challenge: String,
}

impl OauthProviderModule {
    pub(crate) fn insert_open(&self, request: OauthRequest) -> Uuid {
        let mut guard = self
            .open_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let uuid = Uuid::new_v4();
        guard.insert(uuid, request);
        uuid
    }

    pub(crate) fn remove_open(&self, request_uuid: Uuid) -> Option<OauthRequest> {
        let mut guard = self
            .open_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        guard.remove(&request_uuid)
    }

    pub(crate) fn insert_accepted(&self, request: OauthRequest) -> Uuid {
        let mut guard = self
            .accepted_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let uuid = Uuid::new_v4();
        guard.insert(uuid, request);
        uuid
    }
}

impl Module for OauthProviderModule {
    type Setup = OauthProviderSetup;
    type PreInit = PreInit;

    async fn pre_init(setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(PreInit { setup })
    }

    type Dependencies = (Database,);

    async fn init(
        pre_init: Self::PreInit,
        (db,): &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Self {
            db: db.clone(),
            setup: pre_init.setup,
            open_requests: Mutex::new(HashMap::new()),
            accepted_requests: Mutex::new(HashMap::new()),
        })
    }
}

pub struct PreInit {
    setup: OauthProviderSetup,
}
