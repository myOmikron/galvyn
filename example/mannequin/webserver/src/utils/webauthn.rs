//! Utilities for working with webauthn which are shared among multiple groups of handlers

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use webauthn_rs::prelude::WebauthnError;

/// The result when registering a new webauthn key
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result")]
pub enum WebAuthnRegisterResult {
    /// The key was registered successfully
    Ok,
    /// The used device is rejected to be used with attestation
    RejectedDevice,
    /// The browser denied the access to device information which is required to check attestation
    MissingDevice,
    // Other errors are mapped to `ApiError::InternalServerError`
}
impl WebAuthnRegisterResult {
    /// Maps some variants of `WebauthnError` into a `WebAuthnRegisterResult`
    ///
    /// (This function will never return `Some(WebAuthnRegisterResult::Ok)`.)
    pub fn parse(error: &WebauthnError) -> Option<Self> {
        Some(match error {
            WebauthnError::AttestationNotVerifiable => Self::MissingDevice,
            _ => return None,
        })
    }
}
