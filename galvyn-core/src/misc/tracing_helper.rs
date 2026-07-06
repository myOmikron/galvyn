//! Value wrapper for `tracing` analogous to [`tracing::field::display`] and [`tracing::field::debug`]

use std::cell::OnceCell;
use std::fmt;
use std::fmt::Formatter;
use std::fmt::Write;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Serialize;
use tracing::Value;
use tracing::field::DebugValue;
use tracing::field::debug;

use crate::misc::redacted::Redacted;

/// Wraps a type implementing `Serialize` as `Value` that can be recorded as a json string
pub fn json<T>(value: &T) -> impl Value + use<'_, T>
where
    T: Serialize + ?Sized,
{
    string_closure(move || {
        let result = Redacted::with_tracing_serialize(|| serde_json::to_string(value));
        result.unwrap_or_else(|_| "<error>".to_string())
    })
}

/// Wraps a byte slice as `Value` that can be recorded as a hex string
pub fn hex(value: &[u8]) -> impl Value + use<'_> {
    string_closure(move || {
        let mut string = String::with_capacity(value.len() * 2);
        for byte in value {
            string
                .write_fmt(format_args!("{byte:02x}"))
                .expect("fmt::Write for String shouldn't fail");
        }
        string
    })
}

/// Wraps a type implementing `Hash` as `Value` that can be recorded as a hex string
///
/// This can be useful if you don't want to log the value because it is sensitive,
/// but you do want to log the value for the purpose of seeing which values are the same.
///
/// For example session cookies and oauth `state`.
///
/// It will use rust's [`DefaultHasher`] (without random seed).
pub fn hash<T>(value: &T) -> impl Value + use<'_, T>
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let hash = hasher.finish().to_ne_bytes();

    let mut string = String::with_capacity(hash.len() * 2);
    for byte in hash {
        string
            .write_fmt(format_args!("{byte:02x}"))
            .expect("fmt::Write for String shouldn't fail");
    }
    string
}

/// Defines a [`Value`] by writing a closure which produces a `String`.
fn string_closure<F>(closure: F) -> DebugValue<StringClosure<F>>
where
    F: Fn() -> String,
{
    debug(StringClosure {
        closure,
        cache: OnceCell::new(),
    })
}

/// [`Value`] impl returned by [`string_closure`]
struct StringClosure<F> {
    /// Closure producing the string to log
    closure: F,

    /// Cache for the closure's result.
    ///
    /// `tracing` is visitor based and a value might be visited a lot
    /// depending on the application's subscriber setup.
    /// We run the potentially expensive™ closure only once by caching its result.
    cache: OnceCell<String>,
}
impl<F> fmt::Debug for StringClosure<F>
where
    F: Fn() -> String,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.cache.get_or_init(&self.closure))
    }
}
