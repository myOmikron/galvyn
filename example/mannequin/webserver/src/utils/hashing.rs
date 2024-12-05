//! Helper methods for hashing are defined in this module

use argon2::password_hash::Error;
use argon2::password_hash::SaltString;
use argon2::Argon2;
use argon2::PasswordHash;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use thiserror::Error;

/// Hash a password
pub fn hash_pw(pw: &str) -> Result<String, argon2::password_hash::Error> {
    Argon2::default()
        .hash_password(
            pw.as_bytes(),
            &SaltString::generate(&mut rand::thread_rng()),
        )
        .map(|x| x.to_string())
}

/// Verify a password
pub fn verify_pw(pw: &str, hash: &str) -> Result<(), VerifyPwError> {
    Argon2::default()
        .verify_password(pw.as_bytes(), &PasswordHash::new(hash)?)
        .map_err(|e| match e {
            Error::Password => VerifyPwError::Mismatch,
            _ => VerifyPwError::Hash(e),
        })?;

    Ok(())
}

/// The possible outcomes of a verify_pw operation
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum VerifyPwError {
    #[error("Hashing error: {0}")]
    Hash(#[from] argon2::password_hash::Error),
    #[error("Password mismatched hash")]
    Mismatch,
}
