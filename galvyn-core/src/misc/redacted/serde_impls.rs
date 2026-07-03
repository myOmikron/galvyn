//! [`serde`] implementations for [`Redacted`]

use std::cell::Cell;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::ser::Error as SerError;

use crate::misc::redacted::Redacted;
use crate::misc::redacted::config;
use crate::misc::redacted::config::PartToRedact;

thread_local! {
    /// "Global" thread which tells `Serialize` if it is run for a `tracing` attribute
    pub static TRACING_SERIALIZE: Cell<bool> = Cell::new(false);
}

impl<T, Config> Serialize for Redacted<T, Config>
where
    T: Serialize,
    Config: config::RedactionConfig,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Select `part_to_redact` based on `TRACING_SERIALIZE`
        //
        // (Pre-mature) Optimization: Only check the global if it would make any difference.
        let part_to_redact = if Config::VALUE.serialize == Config::VALUE.tracing_serialize {
            Config::VALUE.serialize
        } else if TRACING_SERIALIZE.get() {
            Config::VALUE.tracing_serialize
        } else {
            Config::VALUE.serialize
        };

        if matches!(part_to_redact, PartToRedact::Nothing) {
            self.sensitiv.serialize(serializer)
        } else {
            self.sensitiv.serialize(RedactingSerializer {
                inner: serializer,
                part_to_redact,
            })
        }
    }
}

impl<'de, T, Config> Deserialize<'de> for Redacted<T, Config>
where
    T: Deserialize<'de>,
    Config: config::RedactionConfig,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }

    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize_in_place(deserializer, &mut place.sensitiv)
    }
}

/// `Serializer` which **only** supports strings and applies `part_to_redact` unconditionally.
///
/// TODO: for non-strings should it error, panic, passthrough, passthrough & warn
struct RedactingSerializer<S> {
    inner: S,
    part_to_redact: PartToRedact,
}

impl<S> Serializer for RedactingSerializer<S>
where
    S: Serializer,
{
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeTuple = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeTupleStruct = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeTupleVariant = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeMap = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeStruct = serde::ser::Impossible<S::Ok, S::Error>;
    type SerializeStructVariant = serde::ser::Impossible<S::Ok, S::Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `bool`"))
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `i8`"))
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `i16`"))
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `i32`"))
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `i64`"))
    }

    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `i128`"))
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `u8`"))
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `u16`"))
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `u32`"))
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `u64`"))
    }

    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `u128`"))
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `f32`"))
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `f64`"))
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `char`"))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.inner
            .serialize_str(self.part_to_redact.redact(v).as_ref())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `&[u8]`")) // Maybe supportable?
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `none`"))
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(S::Error::custom("unsupported type `some`"))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `unit`"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `unit_struct`"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(S::Error::custom("unsupported type `unit_variant`"))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(S::Error::custom("unsupported type `newtype_struct`"))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(S::Error::custom("unsupported type `newtype_variant`"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(S::Error::custom("unsupported type `seq`"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(S::Error::custom("unsupported type `tuple`"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(S::Error::custom("unsupported type `tuple_struct`"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(S::Error::custom("unsupported type `tuple_variant`"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(S::Error::custom("unsupported type `map`"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(S::Error::custom("unsupported type `struct`"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(S::Error::custom("unsupported type `struct_variant`"))
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}
