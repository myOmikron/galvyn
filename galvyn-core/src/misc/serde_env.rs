//! Deserializes env variables into some collection `T`.

use std::ffi::OsString;

use serde::de::DeserializeOwned;
use serde::de::DeserializeSeed;
use serde::de::IntoDeserializer;
use serde::de::Visitor;
use serde::de::value::StringDeserializer;

use crate::misc::serde_parse::StringParseDeserializer;

/// Deserializes a collection from the environment variables
pub fn from_env<T>() -> Result<T, serde::de::value::Error>
where
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::from_env())
}

/// Deserializes env variables into some collection `T`.
///
/// It supports both map and sequence shaped collections.
pub struct Deserializer {
    /// Environment variables to deserialize from
    pub input: Vec<(String, String)>,
}

impl Deserializer {
    /// Constructs a `Deserializer` from [`std::env::vars_os`]
    pub fn from_env() -> Self {
        let mut this = Self { input: Vec::new() };
        for (k, v) in std::env::vars_os() {
            this.input.push((os_to_utf8_lossy(k), os_to_utf8_lossy(v)));
        }
        this
    }
}

/// Converts an [`OsString`] to an [`String`] replacing invalid utf8
fn os_to_utf8_lossy(value: OsString) -> String {
    value.into_string().unwrap_or_else(|os_string| {
        String::from_utf8_lossy(os_string.as_encoded_bytes()).into_owned()
    })
}

impl<'de> serde::Deserializer<'de> for Deserializer {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
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
        visitor.visit_seq(serde::de::value::SeqDeserializer::new(
            self.input.into_iter().map(KeyValue),
        ))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
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
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(serde::de::value::MapDeserializer::new(
            self.input
                .into_iter()
                .map(|(k, v)| (StringDeserializer::new(k), StringParseDeserializer::new(v))),
        ))
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
        // TODO: perform "re-casing"?
        // TODO: support nesting through prefixes?
        self.deserialize_map(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit
        bytes byte_buf unit_struct identifier ignored_any option enum
    }
}

/// Adapter which converts to [`KeyValueDeserializer`]
struct KeyValue((String, String));
impl<'de> IntoDeserializer<'de> for KeyValue {
    type Deserializer = KeyValueDeserializer;

    fn into_deserializer(self) -> Self::Deserializer {
        KeyValueDeserializer(Some((Some(self.0.0), self.0.1)))
    }
}

/// `(String, String)` deserializer which uses `StringParseDeserializer` for the 2nd string.
struct KeyValueDeserializer(Option<(Option<String>, String)>);
impl<'de> serde::de::Deserializer<'de> for KeyValueDeserializer {
    type Error = serde::de::value::Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
impl<'de> serde::de::SeqAccess<'de> for KeyValueDeserializer {
    type Error = serde::de::value::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.0.take() {
            Some((Some(key), value)) => {
                self.0 = Some((None, value));
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            Some((None, value)) => {
                self.0 = None;
                seed.deserialize(StringParseDeserializer::new(value))
                    .map(Some)
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::Deserializer;

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestNoCase {
        int: i32,
        uint: u8,
        float: f64,
        r#bool: bool,
        string: String,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "UPPERCASE")]
    struct TestUpperCase {
        int: i32,
        uint: u8,
        float: f64,
        r#bool: bool,
        string: String,
    }

    #[test]
    fn test_from_env() {
        assert_eq!(
            TestNoCase::deserialize(Deserializer {
                input: [
                    ("int", "-1337"),
                    ("uint", "137"),
                    ("float", "1337.0"),
                    ("bool", "y"),
                    ("string", "1337"),
                    ("ignored", "foo")
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
            })
            .unwrap(),
            TestNoCase {
                int: -1337,
                uint: 137,
                float: 1337.0,
                r#bool: true,
                string: "1337".to_string(),
            }
        );

        assert_eq!(
            TestUpperCase::deserialize(Deserializer {
                input: [
                    ("INT", "-1337"),
                    ("UINT", "137"),
                    ("FLOAT", "1337.0"),
                    ("BOOL", "y"),
                    ("STRING", "1337"),
                    ("ignored", "foo")
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
            })
            .unwrap(),
            TestUpperCase {
                int: -1337,
                uint: 137,
                float: 1337.0,
                r#bool: true,
                string: "1337".to_string(),
            }
        );
    }
}
