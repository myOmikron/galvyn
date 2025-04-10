use crate::handler;
#[cfg(feature = "oidc")]
use crate::logic::oidc;
use galvyn_core::{GalvynRouter, InitError, Module, PreInitError};
#[cfg(feature = "oidc")]
use openidconnect::{ClientId, ClientSecret, IssuerUrl, RedirectUrl};
use rorm::Database;
use serde::{Deserialize, Serialize};
use std::future::{ready, Future};
use std::path::PathBuf;
use std::{fs, io};
use webauthn_rs::prelude::{AttestationCaList, Url};
use webauthn_rs::{Webauthn, WebauthnBuilder};

/// The authentication module provides the state required by the authentication handlers
pub struct AuthModule {
    pub handler: AuthHandler,
    pub(crate) db: Database,
    #[cfg(feature = "oidc")]
    pub(crate) oidc: oidc::Client,
    pub(crate) webauthn: Webauthn,
    pub(crate) attestation_ca_list: AttestationCaList,
}

#[derive(Debug, Default)]
pub struct AuthSetup {
    private: (),
}

#[derive(Default, Copy, Clone)]
#[non_exhaustive]
pub struct AuthHandler {
    pub logout: handler::core::logout,

    #[cfg(feature = "oidc")]
    pub login_oidc: handler::oidc::login_oidc,
    #[cfg(feature = "oidc")]
    pub finish_login_oidc: handler::oidc::finish_login_oidc,

    pub login_local_webauthn: handler::local::login_local_webauthn,
    pub finish_login_local_webauthn: handler::local::finish_login_local_webauthn,
    pub login_local_password: handler::local::login_local_password,
    pub set_local_password: handler::local::set_local_password,
    pub delete_local_password: handler::local::delete_local_password,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthConfig {
    #[cfg(feature = "oidc")]
    pub oidc_issuer_url: IssuerUrl,
    #[cfg(feature = "oidc")]
    pub oidc_client_id: ClientId,
    #[cfg(feature = "oidc")]
    pub oidc_client_secret: ClientSecret,
    #[cfg(feature = "oidc")]
    pub oidc_redirect_url: RedirectUrl,

    pub webauthn_id: String,
    pub webauthn_origin: Url,
    pub webauthn_attestation_ca_list: PathBuf,
}

impl AuthHandler {
    pub fn as_router(&self) -> GalvynRouter {
        let router = GalvynRouter::new()
            .handler(self.logout)
            .handler(self.login_local_webauthn)
            .handler(self.finish_login_local_webauthn)
            .handler(self.login_local_password)
            .handler(self.set_local_password)
            .handler(self.delete_local_password);

        #[cfg(feature = "oidc")]
        let router = router
            .handler(self.login_oidc)
            .handler(self.finish_login_oidc);

        router
    }
}

pub struct AuthPreInit {
    #[cfg(feature = "oidc")]
    oidc: oidc::Client,
    webauthn: Webauthn,
    attestation_ca_list: AttestationCaList,
}

impl Module for AuthModule {
    type Setup = AuthSetup;

    type PreInit = AuthPreInit;

    fn pre_init(
        AuthSetup { private: () }: Self::Setup,
    ) -> impl Future<Output = Result<Self::PreInit, PreInitError>> + Send {
        async move {
            let auth_config: AuthConfig = envy::from_env()?;

            #[cfg(feature = "oidc")]
            let oidc = oidc::Client::discover(oidc::Config {
                url: auth_config.oidc_issuer_url,
                client_id: auth_config.oidc_client_id,
                client_secret: auth_config.oidc_client_secret,
                redirect_url: auth_config.oidc_redirect_url, // TODO try to calculate this ourselves
            })
            .await?;

            let webauthn =
                WebauthnBuilder::new(&auth_config.webauthn_id, &auth_config.webauthn_origin)?
                    .build()?;
            let attestation_ca_list = serde_json::from_reader(io::BufReader::new(fs::File::open(
                &auth_config.webauthn_attestation_ca_list,
            )?))?;

            Ok(AuthPreInit {
                #[cfg(feature = "oidc")]
                oidc,
                webauthn,
                attestation_ca_list,
            })
        }
    }

    type Dependencies = (Database,);

    fn init(
        pre_init: Self::PreInit,
        (db,): &mut Self::Dependencies,
    ) -> impl Future<Output = Result<Self, InitError>> + Send {
        ready(Ok(Self {
            db: db.clone(),
            #[cfg(feature = "oidc")]
            oidc: pre_init.oidc,
            webauthn: pre_init.webauthn,
            attestation_ca_list: pre_init.attestation_ca_list,
            handler: AuthHandler::default(),
        }))
    }
}
