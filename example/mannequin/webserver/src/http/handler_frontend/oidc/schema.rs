//! The schemas for oidc handlers

use openidconnect::core::CoreIdTokenClaims;
use openidconnect::core::CoreTokenResponse;
use openidconnect::AuthorizationCode;
use openidconnect::CsrfToken;
use openidconnect::Nonce;
use openidconnect::PkceCodeVerifier;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::utils::schemars::SchemaString;

#[derive(Serialize, Deserialize, Debug)]
#[allow(missing_docs)]
pub struct AuthState {
    pub csrf_token: CsrfToken,
    pub pkce_code_verifier: PkceCodeVerifier,
    pub nonce: Nonce,
}

#[derive(Deserialize, JsonSchema)]
#[allow(missing_docs)]
pub struct AuthRequest {
    pub code: SchemaString<AuthorizationCode>,
    pub state: SchemaString<CsrfToken>,
}

/// Data the [`super::handler_common::finish_login`] handler will store in the user's session
#[derive(Serialize, Deserialize)]
pub struct UserData {
    /// The oidc token
    pub token: CoreTokenResponse,

    /// The OIDC claims
    pub claims: CoreIdTokenClaims,
}
