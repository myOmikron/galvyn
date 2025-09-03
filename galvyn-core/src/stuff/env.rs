//! Helper for reading configuration from env variables

use std::env;
use std::env::VarError;
use std::fmt;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::OnceLock;

use serde::Deserializer;
use serde::de::DeserializeOwned;
use serde::de::Error;
use serde::de::Visitor;
use thiserror::Error;

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
                match T::deserialize(StringDeserializer(value)) {
                    Ok(value) => Ok(value),
                    Err(StringDeserializerError(error)) => match self.default {
                        Some(default) if is_empty => Ok(default()),
                        _ => Err(EnvError {
                            name: self.name,
                            reason: EnvErrorReason::Malformed(error),
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

/// An improved [`StringDeserializer`](serde::de::value::StringDeserializer)
pub struct StringDeserializer(pub String);

/// Error produced by [`StringDeserializer`]
#[derive(Debug, Error)]
#[error("{0}")]
pub struct StringDeserializerError(pub String);

impl<'de> Deserializer<'de> for StringDeserializer {
    type Error = StringDeserializerError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.0.as_str() {
            "true" | "1" | "yes" | "y" => visitor.visit_bool(true),
            "false" | "0" | "no" | "n" => visitor.visit_bool(false),
            _ => visitor.visit_string(self.0),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.0.parse().map_err(Self::Error::custom)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut chars = self.0.chars();
        #[expect(clippy::collapsible_if, reason = "Only allowed on newer rust versions")]
        if let Some(ch) = chars.next() {
            if chars.next().is_none() {
                return visitor.visit_char(ch);
            }
        }
        visitor.visit_string(self.0)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }
}

impl Error for StringDeserializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self(msg.to_string())
    }
}
