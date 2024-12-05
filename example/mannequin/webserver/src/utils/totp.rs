//! Small helper functions to construct [`TOTP`]s in a consistent way

use thiserror::Error;
use totp_rs::Rfc6238;
use totp_rs::Rfc6238Error;
use totp_rs::TotpUrlError;
use totp_rs::TOTP;

use crate::utils::checked_string::CheckedString;
use crate::utils::secure_string::SecureString;

/// Constructs a [`TOTP`] from an unencoded secret
pub fn totp_from_binary(secret: Vec<u8>) -> Result<TOTP, TotpFromError> {
    Ok(TOTP::from_rfc6238(Rfc6238::with_defaults(secret)?)?)
}

/// Constructs a [`TOTP`] from a base32 encoded secret
pub fn totp_from_base32(
    secret: &CheckedString<32, 64, SecureString>,
) -> Result<TOTP, TotpFromError> {
    let Some(secret) = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret) else {
        return Err(TotpFromError::InvalidBase32);
    };
    totp_from_binary(secret)
}

/// Error returned by [`totp_from_binary`] and [`totp_from_base32`]
#[derive(Error, Debug)]
pub enum TotpFromError {
    /// The secret is not compliant to [rfc-6238](https://tools.ietf.org/html/rfc6238).
    #[error("Invalid secret")]
    InvalidSecret(#[from] Rfc6238Error),

    /// The string passed to [`totp_from_base32`] wasn't valid base32
    #[error("Invalid base32")]
    InvalidBase32,

    /// `TOTP::from_rfc6238` returns a `Result` but looking at its source code, it is never returned.
    /// This error variant exists to catch a change in that behaviour across versions.
    /// It also serves to report this bug through our logging pipeline instead of panicking.
    #[error("This should never happen: creating `TOTP` from valid `Rfc6238` cannot fail")]
    Unreachable(#[from] TotpUrlError),
}
