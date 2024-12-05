use galvyn_core::{InitError, Module, PreInitError};
use openidconnect::core::{CoreClient as OidcClient, CoreProviderMetadata};
use openidconnect::reqwest::async_http_client;
use openidconnect::{ClientId, ClientSecret, IssuerUrl};
use rorm::Database;
use serde::{Deserialize, Serialize};
use std::future::{ready, Future};
use std::path::PathBuf;
use std::{fs, io};
use webauthn_rs::prelude::{AttestationCaList, Url};
use webauthn_rs::{Webauthn, WebauthnBuilder};

/// The authentication module provides the state required by the authentication handlers
pub struct AuthModule {
    pub(crate) db: Database,
    pub(crate) oidc: OidcClient,
    pub(crate) webauthn: Webauthn,
    pub(crate) attestation_ca_list: AttestationCaList,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthConfig {
    pub oidc_issuer_url: IssuerUrl,
    pub oidc_client_id: ClientId,
    pub oidc_client_secret: ClientSecret,

    pub webauthn_id: String,
    pub webauthn_origin: Url,
    pub webauthn_attestation_ca_list: PathBuf,
}

impl Module for AuthModule {
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
        }))
    }
}
