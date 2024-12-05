use openidconnect::{AuthorizationCode, CsrfToken};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLoginFlowsRequest {
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum GetLoginFlowsResponse {
    Oidc(OidcLoginFlow),
    Local(LocalLoginFlow),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcLoginFlow {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLoginFlow {
    pub password: bool,
    pub webauthn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FinishLoginOidcRequest {
    pub code: AuthorizationCode,
    pub state: CsrfToken,
}
