use crate::{handler, AuthModels};
use galvyn_core::{GalvynRouter, InitError, Module, PreInitError};
use openidconnect::core::{CoreClient as OidcClient, CoreProviderMetadata};
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

/// The authentication module provides the state required by the authentication handlers
pub struct AuthModule<M: AuthModels> {
    pub handler: AuthHandler<M>,
    pub(crate) db: Database,
    pub(crate) oidc: OidcClient,
    pub(crate) webauthn: Webauthn,
    pub(crate) attestation_ca_list: AttestationCaList,
    models: PhantomData<M>,
}

pub struct AuthHandler<M: AuthModels> {
    pub get_login_flow: handler::get_login_flow<M>,
    pub login_oidc: handler::login_oidc<M>,
    pub finish_login_oidc: handler::finish_login_oidc<M>,
    pub login_local_webauthn: handler::login_local_webauthn<M>,
    pub finish_login_local_webauthn: handler::finish_login_local_webauthn<M>,
    pub login_local_password: handler::login_local_password<M>,
    pub logout: handler::logout,
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
        GalvynRouter::new()
            .handler(self.get_login_flow)
            .handler(self.login_oidc)
            .handler(self.finish_login_oidc)
            .handler(self.login_local_webauthn)
            .handler(self.finish_login_local_webauthn)
            .handler(self.login_local_password)
            .handler(self.logout)
    }
}

impl<M: AuthModels> Module for AuthModule<M> {
    type PreInit = (OidcClient, Webauthn, AttestationCaList);

    fn pre_init() -> impl Future<Output = Result<Self::PreInit, PreInitError>> + Send {
        async move {
            let AuthConfig {
                oidc_issuer_url,
                oidc_client_id,
                oidc_client_secret,
                webauthn_id,
                webauthn_origin,
                webauthn_attestation_ca_list,
            } = envy::from_env()?;

            let oidc = OidcClient::from_provider_metadata(
                CoreProviderMetadata::discover_async(oidc_issuer_url, async_http_client).await?,
                oidc_client_id,
                Some(oidc_client_secret),
            );
            // TODO: can't set redirect uri before application author mounted our handler to its router :(

            let webauthn = WebauthnBuilder::new(&webauthn_id, &webauthn_origin)?.build()?;
            let attestation_ca_list = serde_json::from_reader(io::BufReader::new(fs::File::open(
                &webauthn_attestation_ca_list,
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
                get_login_flow: handler::get_login_flow(PhantomData),
                login_oidc: handler::login_oidc(PhantomData),
                finish_login_oidc: handler::finish_login_oidc(PhantomData),
                login_local_webauthn: handler::login_local_webauthn(PhantomData),
                finish_login_local_webauthn: handler::finish_login_local_webauthn(PhantomData),
                login_local_password: handler::login_local_password(PhantomData),
                logout: handler::logout(PhantomData),
            },
        }))
    }
}
