//! Helper for reading configuration from env variables

use std::env;
use std::env::VarError;
use std::fmt;
use std::ops::Deref;
use std::sync::OnceLock;

use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::misc::serde_parse::StringParseDeserializer;

/// An environment variable
///
/// # Example
///
/// ```rust
/// # use galvyn_core::stuff::env::EnvVar;
/// #
/// // A required environment variable
/// static DB_PWD: EnvVar = EnvVar::required("DB_PWD");
///
/// // An optional environment variable
/// static DB_HOST: EnvVar = EnvVar::optional("HOST", || "localhost".to_string());
///
/// // An environment variable of a non-string type
/// static DB_PORT: EnvVar<u16> = EnvVar::optional("PORT", || 5432);
pub struct EnvVar<T = String> {
    /// The read and deserialized value
    value: OnceLock<Result<T, EnvError>>,

    /// The environment variable to read
    name: &'static str,

    /// A function which produces a default value.
    ///
    /// The default is used if the variable is not set.
    /// If this field ìs `None`, then the variable is required to be set.
    default: Option<fn() -> T>,
}

impl<T: DeserializeOwned> EnvVar<T> {
    /// Constructs an environment variable which is required
    pub const fn required(name: &'static str) -> Self {
        Self {
            name,

            value: OnceLock::new(),
            default: None,
        }
    }

    /// Constructs an environment variable which is optional and has a default
    pub const fn optional(name: &'static str, default: fn() -> T) -> Self {
        Self {
            name,

            value: OnceLock::new(),
            default: Some(default),
        }
    }

    /// Gets the environment variable's value (or its default)
    ///
    /// # Panics
    /// If the variable could not be read and parsed
    pub fn get(&self) -> &T {
        self.try_get().unwrap_or_else(|error| panic!("{error}"))
    }

    /// Loads the environment variable's value returning a possible error
    pub fn load(&self) -> Result<(), &EnvError> {
        self.try_get().map(|_| ())
    }

    /// Gets the environment variable's value (or its default)
    pub fn try_get(&self) -> Result<&T, &EnvError> {
        self.value
            .get_or_init(|| {
                let value = match env::var(self.name) {
                    Ok(value) => value,
                    Err(VarError::NotUnicode(_)) => {
                        return Err(EnvError {
                            name: self.name,
                            reason: EnvErrorReason::NotUtf8,
                        });
                    }
                    Err(VarError::NotPresent) => {
                        return match self.default {
                            None => Err(EnvError {
                                name: self.name,
                                reason: EnvErrorReason::Missing,
                            }),
                            Some(default) => Ok(default()),
                        };
                    }
                };
                let is_empty = value.is_empty();
                match T::deserialize(StringParseDeserializer(value)) {
                    Ok(value) => Ok(value),
                    Err(error) => match self.default {
                        Some(default) if is_empty => Ok(default()),
                        _ => Err(EnvError {
                            name: self.name,
                            reason: EnvErrorReason::Malformed(error.to_string()),
                        }),
                    },
                }
            })
            .as_ref()
    }
}

impl<T: DeserializeOwned> Deref for EnvVar<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: DeserializeOwned + fmt::Display> fmt::Display for EnvVar<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// Error while reading and parsing an environment variable
#[derive(Debug, Error, Clone)]
#[error("Environment variable '{name}' is {reason}")]
pub struct EnvError {
    /// The environment varible which cause this error
    pub name: &'static str,

    /// The reason why the environment variable couldn't be read
    pub reason: EnvErrorReason,
}

/// The reason why an environment variable couldn't be read
#[derive(Debug, Error, Clone)]
pub enum EnvErrorReason {
    /// Variable is not set
    #[error("not set")]
    Missing,

    /// Failed to decode the variable's value
    #[error("not utf8")]
    NotUtf8,

    /// Failed to parse the variable's value
    #[error("malformed: {0}")]
    Malformed(String),
}
