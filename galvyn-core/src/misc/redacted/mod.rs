//! New-types to hide values from logs

use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::panic::resume_unwind;

pub use self::config::After;
pub use self::config::Fully;
use crate::misc::redacted::serde_impls::TRACING_SERIALIZE;

pub mod config;
mod serde_impls;
mod std_impls;

/// New-type which wraps a sensitive value to hide it from logs.
///
/// # Why
///
/// This is useful to mark sensitive values in the typesystem and prevent accidental leaks to logging.
///
/// A sensitive value might for example be a secret or a person's data protected by law.
///
/// # How
///
/// This type overwrite traits commonly used for logging of any nature.
///
/// At the moment it overwrites:
/// - [`std::fmt::Debug`]
/// - [`std::fmt::Display`]
/// - [`serde::Serialize`] (when used to produce a JSON representation for logging)
pub struct Redacted<T, Config = Fully> {
    /// The protected sensitive value
    ///
    /// This field is public but has a very explicit name.
    /// Accessing it is not directly dangerous but should be done with care, not to leak it.
    ///
    /// # Don't:
    ///
    /// ```rust
    /// # use galvyn_core::misc::redacted::Redacted;
    /// #
    /// let secret = Redacted::new("very secret");
    /// println!("{}", secret.sensitive); // Leak to stdout
    /// ```
    ///
    /// # Do:
    ///
    /// ```rust
    /// # use galvyn_core::misc::redacted::Redacted;
    /// #
    /// let secret = Redacted::new("very secret");
    /// is_valid(&secret.sensitive); // Check the actual value
    /// // Of course, `is_valid` should not leak the secret either.
    /// // Ideally you carry the `Redacted` type through your code as long as possible.
    /// ```
    pub sensitive: T,
    config: PhantomData<Config>,
}

impl<T, Config> Redacted<T, Config>
where
    Config: config::RedactionConfig,
{
    /// Wraps a `value` to redact its `Debug` and `Display` (and more) implementations.
    ///
    /// What trait is redacted to what degree is subject to the generic `Config`.
    pub const fn new(value: T) -> Self {
        Self {
            sensitive: value,
            config: PhantomData,
        }
    }
}

impl Redacted<()> {
    /// Use `RedactionConfig.tracing_serialize` instead of `RedactionConfig.serialize` for the body of a closure.
    ///
    /// This function is probably not want you are looking for.
    pub fn with_tracing_serialize<R>(f: impl FnOnce() -> R) -> R {
        TRACING_SERIALIZE.set(true);
        let result = catch_unwind(AssertUnwindSafe(f));
        TRACING_SERIALIZE.set(false);
        match result {
            Ok(x) => x,
            Err(x) => resume_unwind(x),
        }
    }
}
