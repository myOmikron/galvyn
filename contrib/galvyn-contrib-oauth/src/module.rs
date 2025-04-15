use galvyn_core::re_exports::rorm::Database;
use galvyn_core::{InitError, Module, PreInitError};

pub struct OauthProviderModule {
    pub(crate) db: Database,
}

#[derive(Debug, Default)]
pub struct OauthProviderSetup {
    private: (),
}

impl Module for OauthProviderModule {
    type Setup = OauthProviderSetup;
    type PreInit = ();

    async fn pre_init(_setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(())
    }

    type Dependencies = (Database,);

    async fn init((): Self::PreInit, (db,): &mut Self::Dependencies) -> Result<Self, InitError> {
        Ok(Self { db })
    }
}
