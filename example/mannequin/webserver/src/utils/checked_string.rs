//! Wrapper around [String] that requires not to be empty

use std::borrow::Borrow;
use std::borrow::Cow;
use std::ops::Deref;

use rorm::fields::traits::FieldEq;
use rorm::FieldAccess;
use schemars::gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use schemars::schema::StringValidation;
use schemars::JsonSchema;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

/// Wrapper around a `T`, most likely a [`String`], which checks the
/// minimum and maximum length of the string
///
/// A `MAX_LEN` of `0` will disable it.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CheckedString<const MIN_LEN: u32 = 0, const MAX_LEN: u32 = 255, T = String>(T)
where
    T: Deref<Target = str>;

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    /// Hacky compile time check, if `MAX_LEN >= MIN_LEN`
    /// (except for `MAX_LEN = 0` which disables it)
    const CHECK: () = {
        if MAX_LEN != 0 && MIN_LEN > MAX_LEN {
            panic!("MIN_LEN > MAX_LEN");
        }
    };

    /// Instantiate a new `CheckedString`
    pub fn new(str: T) -> Result<Self, ConstraintsViolated> {
        let len = str.len() as u32;
        if len < MIN_LEN {
            return Err(ConstraintsViolated::TooShort {
                min: MIN_LEN,
                got: str.len(),
            });
        }
        if MAX_LEN > 0 && len > MAX_LEN {
            return Err(ConstraintsViolated::TooLong {
                max: MIN_LEN,
                got: str.len(),
            });
        }

        // "Use" `Self::CHECK` to force the compiler to evaluate its block
        let _check = Self::CHECK;

        Ok(Self(str))
    }

    /// Convert the string into its inner type
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// String passed to [`CheckedString::new`] is either too long or too short
#[derive(Debug, Error)]
pub enum ConstraintsViolated {
    /// The string passed to [`CheckedString::new`] was too long
    #[error("Maximum of {max} violated: {got} > {max}")]
    TooLong {
        /// The exceeded maximum length
        max: u32,
        /// The exceeding length
        got: usize,
    },

    /// The string passed to [`CheckedString::new`] was too short
    #[error("Minimum of {min} violated: {got} < {min}")]
    TooShort {
        /// The exceeded minimum length
        min: u32,
        /// The exceeding length
        got: usize,
    },
}

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> Deref for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> Borrow<str> for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> Borrow<T> for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> AsRef<T> for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

// ------------ //
//   schemars   //
// ------------ //

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> JsonSchema for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    fn is_referenceable() -> bool {
        false
    }
    fn schema_name() -> String {
        format!("CheckedString_{MIN_LEN}_{MAX_LEN}")
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Owned(format!("CheckedString<{MIN_LEN}, {MAX_LEN}>",))
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: None,
            string: Some(Box::new(StringValidation {
                max_length: if MAX_LEN == 0 { None } else { Some(MAX_LEN) },
                min_length: Some(MIN_LEN),
                pattern: None,
            })),
            ..Default::default()
        }
        .into()
    }
}

// --------- //
//   serde   //
// --------- //

impl<'de, const MIN_LEN: u32, const MAX_LEN: u32, T> Deserialize<'de>
    for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str> + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = T::deserialize(deserializer)?;
        let checked = Self::new(string).map_err(Error::custom)?;
        Ok(checked)
    }
}

impl<const MIN_LEN: u32, const MAX_LEN: u32, T> Serialize for CheckedString<MIN_LEN, MAX_LEN, T>
where
    T: Deref<Target = str>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

// -------- //
//   rorm   //
// -------- //
impl<'rhs, const MIN_LEN: u32, const MAX_LEN: u32, T>
    FieldEq<'rhs, &'rhs CheckedString<MIN_LEN, MAX_LEN, T>> for String
where
    T: Deref<Target = str>,
{
    type EqCond<A: FieldAccess> = <String as FieldEq<'rhs, &'rhs str>>::EqCond<A>;
    fn field_equals<A: FieldAccess>(
        access: A,
        value: &'rhs CheckedString<MIN_LEN, MAX_LEN, T>,
    ) -> Self::EqCond<A> {
        <String as FieldEq<'rhs, &'rhs str>>::field_equals(access, &value)
    }

    type NeCond<A: FieldAccess> = <String as FieldEq<'rhs, &'rhs str>>::NeCond<A>;
    fn field_not_equals<A: FieldAccess>(
        access: A,
        value: &'rhs CheckedString<MIN_LEN, MAX_LEN, T>,
    ) -> Self::NeCond<A> {
        <String as FieldEq<'rhs, &'rhs str>>::field_not_equals(access, &value)
    }
}
