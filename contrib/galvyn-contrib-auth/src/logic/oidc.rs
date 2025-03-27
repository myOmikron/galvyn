use std::time::Duration;

use galvyn_core::stuff::api_error::{ApiError, ApiResult};
use galvyn_core::stuff::schema::ApiStatusCode;
use openidconnect::core::CoreAuthenticationFlow;
use openidconnect::core::CoreClient;
use openidconnect::core::CoreIdTokenClaims;
use openidconnect::core::CoreProviderMetadata;
use openidconnect::reqwest;
use openidconnect::url::Url;
use openidconnect::AccessTokenHash;
use openidconnect::AuthorizationCode;
use openidconnect::ClientId;
use openidconnect::ClientSecret;
use openidconnect::CsrfToken;
use openidconnect::DiscoveryError;
use openidconnect::EndpointMaybeSet;
use openidconnect::EndpointNotSet;
use openidconnect::EndpointSet;
use openidconnect::HttpClientError;
use openidconnect::IssuerUrl;
use openidconnect::Nonce;
use openidconnect::OAuth2TokenResponse;
use openidconnect::PkceCodeChallenge;
use openidconnect::PkceCodeVerifier;
use openidconnect::RedirectUrl;
use openidconnect::RequestTokenError;
use openidconnect::Scope;
use openidconnect::TokenResponse;
use serde::Deserialize;
use serde::Serialize;

pub struct Config {
    pub url: IssuerUrl,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub redirect_url: RedirectUrl,
}

pub struct Client {
    http_client: reqwest::Client,
    oidc_client: OidcClient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcSessionState {
    pub csrf_token: CsrfToken,
    pub pkce_code_verifier: PkceCodeVerifier,
    pub nonce: Nonce,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcRequestState {
    pub code: AuthorizationCode,
    pub state: CsrfToken,
}

pub type DiscoverError = DiscoveryError<HttpClientError<reqwest::Error>>;

type OidcClient = CoreClient<
    EndpointSet, // Auth URL
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet, // Token URL
    EndpointMaybeSet,
>;

impl Client {
    pub async fn discover(config: Config) -> Result<Self, DiscoverError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        let oidc_client = config.discover(&http_client).await?;

        Ok(Self {
            http_client,
            oidc_client,
        })
    }

    pub fn begin_login(&self) -> ApiResult<(Url, OidcSessionState)> {
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL to which we'll redirect the user.
        let request = self
            .oidc_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_code_challenge)
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()));
        let (auth_url, csrf_token, nonce) = request.url();

        Ok((
            auth_url,
            OidcSessionState {
                csrf_token,
                nonce,
                pkce_code_verifier,
            },
        ))
    }

    pub async fn finish_login(
        &self,
        session: OidcSessionState,
        request: OidcRequestState,
    ) -> ApiResult<CoreIdTokenClaims> {
        // Check the states to match
        if request.state != session.csrf_token {
            return Err(ApiError::new(
                ApiStatusCode::Unauthenticated,
                "Secret state is invalid",
            ));
        }

        // Exchange the authorization code with a token.
        let token_response = self
            .oidc_client
            .exchange_code(request.code)
            .set_pkce_verifier(session.pkce_code_verifier)
            .request_async(&self.http_client)
            .await
            .map_err(|error| {
                let code = match error {
                    RequestTokenError::ServerResponse(_) => ApiStatusCode::InternalServerError,
                    _ => ApiStatusCode::Unauthenticated,
                };
                ApiError::new(code, "Failed to exchange code").with_source(error)
            })?;

        // Extract the ID token claims after verifying its authenticity and nonce.
        let id_token = token_response.id_token().ok_or(ApiError::server_error(
            "Oidc provider did not provider an id token. \
            This would suggest its not providing oidc.",
        ))?;
        let id_token_verifier = self.oidc_client.id_token_verifier();
        let claims = id_token
            .claims(&id_token_verifier, &session.nonce)
            .map_err(|error| {
                ApiError::new(ApiStatusCode::Unauthenticated, "Failed to verify id token")
                    .with_source(error)
            })?;

        // Verify the access token hash to ensure that the access token hasn't been substituted for
        // another user's.
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg().map_err(ApiError::map_server_error(
                    "Failed to retrieve signing algorithm",
                ))?,
                id_token
                    .signing_key(&id_token_verifier)
                    .map_err(ApiError::map_server_error("Failed to retrieve signing key"))?,
            )
            .map_err(ApiError::map_server_error(
                "Failed to recreate access token signature",
            ))?;
            if actual_access_token_hash != *expected_access_token_hash {
                return Err(ApiError::new(
                    ApiStatusCode::Unauthenticated,
                    "Invalid access token",
                ));
            }
        }

        Ok(claims.clone())
    }
}

impl Config {
    // async fn discover_retry<const N: usize>(
    //     &self,
    //     http_client: &reqwest::Client,
    // ) -> Result<OidcClient, DiscoveryError<HttpClientError<reqwest::Error>>> {
    //     let mut result = Err(DiscoveryError::Other(String::new()));
    //     for _ in 0..N {
    //         result = self.discover(http_client).await;
    //         if let Err(DiscoveryError::Request(HttpClientError::Reqwest(error))) = &result {
    //             if error.is_timeout() {
    //                 warn!("Timed out fetching oidc discovery, trying again...");
    //                 continue;
    //             }
    //         }
    //         return result;
    //     }
    //     error!("Timed out fetching oidc discovery");
    //     result
    // }

    async fn discover(
        &self,
        http_client: &reqwest::Client,
    ) -> Result<OidcClient, DiscoveryError<HttpClientError<reqwest::Error>>> {
        let oidc_client = CoreClient::from_provider_metadata(
            CoreProviderMetadata::discover_async(self.url.clone(), http_client).await?,
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(self.redirect_url.clone());

        // Check the token url to be set
        let token_uri = oidc_client
            .token_uri()
            .ok_or_else(|| DiscoveryError::Other("Issuer did not provide a token url".to_string()))?
            .clone();
        let oidc_client = oidc_client.set_token_uri(token_uri);

        Ok(oidc_client)
    }
}
