use crate::{handler, AuthModels};
use galvyn_core::{GalvynRouter, InitError, Module, PreInitError};
#[cfg(feature = "oidc")]
use openidconnect::core::{CoreClient as OidcClient, CoreProviderMetadata};
#[cfg(feature = "oidc")]
use openidconnect::reqwest::async_http_client;
use openidconnect::{ClientId, ClientSecret, IssuerUrl};
use rorm::Database;
use serde::{Deserialize, Serialize};
use std::future::{ready, Future};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::{fs, io};
use webauthn_rs::prelude::{AttestationCaList, Url};
use webauthn_rs::{Webauthn, WebauthnBuilder};

#[cfg(not(feature = "oidc"))]
type OidcClient = ();

/// The authentication module provides the state required by the authentication handlers
pub struct AuthModule<M: AuthModels> {
    pub handler: AuthHandler<M>,
    pub(crate) db: Database,
    #[cfg_attr(not(feature = "oidc"), allow(unused))]
    pub(crate) oidc: OidcClient,
    pub(crate) webauthn: Webauthn,
    pub(crate) attestation_ca_list: AttestationCaList,
    models: PhantomData<M>,
}

#[non_exhaustive]
pub struct AuthHandler<M: AuthModels> {
    pub get_login_flow: handler::get_login_flow<M>,
    pub logout: handler::logout,

    #[cfg(feature = "oidc")]
    pub login_oidc: handler::login_oidc<M>,
    #[cfg(feature = "oidc")]
    pub finish_login_oidc: handler::finish_login_oidc<M>,
    #[cfg(not(feature = "oidc"))]
    #[allow(unused)]
    login_oidc: (),
    #[cfg(not(feature = "oidc"))]
    #[allow(unused)]
    finish_login_oidc: (),

    pub login_local_webauthn: handler::login_local_webauthn<M>,
    pub finish_login_local_webauthn: handler::finish_login_local_webauthn<M>,
    pub login_local_password: handler::login_local_password<M>,
    pub set_local_password: handler::set_local_password<M>,
    pub delete_local_password: handler::delete_local_password<M>,
}

impl<M: AuthModels> Clone for AuthHandler<M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M: AuthModels> Copy for AuthHandler<M> {}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthConfig {
    pub oidc_issuer_url: IssuerUrl,
    pub oidc_client_id: ClientId,
    pub oidc_client_secret: ClientSecret,

    pub webauthn_id: String,
    pub webauthn_origin: Url,
    pub webauthn_attestation_ca_list: PathBuf,
}

impl<M: AuthModels> AuthHandler<M> {
    pub fn as_router(&self) -> GalvynRouter {
        let router = GalvynRouter::new()
            .handler(self.get_login_flow)
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

impl<M: AuthModels> Module for AuthModule<M> {
    type PreInit = (OidcClient, Webauthn, AttestationCaList);

    fn pre_init() -> impl Future<Output = Result<Self::PreInit, PreInitError>> + Send {
        async move {
            let auth_config: AuthConfig = envy::from_env()?;

            #[cfg(not(feature = "oidc"))]
            let oidc = ();
            #[cfg(feature = "oidc")]
            let oidc = OidcClient::from_provider_metadata(
                CoreProviderMetadata::discover_async(
                    auth_config.oidc_issuer_url,
                    async_http_client,
                )
                .await?,
                auth_config.oidc_client_id,
                Some(auth_config.oidc_client_secret),
            );
            // TODO: can't set redirect uri before application author mounted our handler to its router :(

            let webauthn =
                WebauthnBuilder::new(&auth_config.webauthn_id, &auth_config.webauthn_origin)?
                    .build()?;
            let attestation_ca_list = serde_json::from_reader(io::BufReader::new(fs::File::open(
                &auth_config.webauthn_attestation_ca_list,
            )?))?;

            Ok((oidc, webauthn, attestation_ca_list))
        }
    }

    type Dependencies = (Database,);

    fn init(
        (oidc, webauthn, attestation_ca_list): Self::PreInit,
        (db,): &mut Self::Dependencies,
    ) -> impl Future<Output = Result<Self, InitError>> + Send {
        ready(Ok(Self {
            db: db.clone(),
            oidc,
            webauthn,
            attestation_ca_list,
            models: PhantomData,
            handler: AuthHandler {
                get_login_flow: Default::default(),
                logout: Default::default(),

                login_oidc: Default::default(),
                finish_login_oidc: Default::default(),

                login_local_webauthn: Default::default(),
                finish_login_local_webauthn: Default::default(),
                login_local_password: Default::default(),
                set_local_password: Default::default(),
                delete_local_password: Default::default(),
            },
        }))
    }
}
