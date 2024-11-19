use galvyn_core::re_exports::swaggapi::re_exports::schemars::JsonSchema;
use galvyn_core::utils::checked_string::CheckedString;
use serde::Deserialize;
use serde::Serialize;

/// The request for to retrieve a user's possible login flows
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginFlowsRequest {
    /// The mail whose login flows to query
    pub mail: CheckedString<1, 255>,
}

/// Flags indicating which login flows are supported by an email's account.
///
/// If `oidc` is `true`, the others have to be `false`.
/// If `oidc` is `false`, at least one of the others has to be `true`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SupportedLoginFlows {
    /// The mail the login flows are for
    pub mail: CheckedString<1, 255>,

    /// Is this email authenticated through OpenId Connect?
    pub oidc: bool,

    /// Does this email support password login?
    pub password: bool,

    /// Does this email support password-less login through a security key?
    pub key: bool,
}
