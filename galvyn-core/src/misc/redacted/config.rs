//! [`RedactionConfig`] and some implementations

use std::borrow::Cow;

#[cfg(doc)]
use crate::misc::redacted::Redacted;

/// Redact the entire sensitive value
///
/// This marker is a [`RedactionConfig`] used in combination with [`Redacted`].
pub struct Fully;
impl RedactionConfig for Fully {
    const VALUE: RedactionConfigValue = RedactionConfigValue {
        debug: PartToRedact::Everything,
        display: PartToRedact::Everything,
        serialize: PartToRedact::Nothing,
        tracing_serialize: PartToRedact::Everything,
    };
}

/// Redact the sensitive value after the first `N` bytes.
///
/// This marker is a [`RedactionConfig`] used in combination with [`Redacted`].
pub struct After<const N: usize>;
impl<const N: usize> RedactionConfig for After<N> {
    const VALUE: RedactionConfigValue = RedactionConfigValue {
        debug: PartToRedact::End { after: N },
        display: PartToRedact::End { after: N },
        serialize: PartToRedact::Nothing,
        tracing_serialize: PartToRedact::End { after: N },
    };
}

/// Which traits and to what extent are redacted by [`Redacted`]?
///
/// This trait is implemented by marker types representing a specific config.
///
/// Ideally `Redacted` would be generic over some `<const CONFIG: RedactionConfig>`.
/// However, this is not supported by stable rust (yet).
///
/// This trait exists as an intermediate.
/// It should only be implemented by zero sized types,
/// and it associates them with a constant instance of [`RedactionConfigValue`].
pub trait RedactionConfig {
    /// Config associated with this marker type
    const VALUE: RedactionConfigValue;
}

/// Which traits and to what extent are redacted by [`Redacted`]?
///
/// Each field represents a specific trait (or circumstance)
/// and which redaction should apply to it.
pub struct RedactionConfigValue {
    /// Which part of the value's [`Debug`](std::fmt::Debug) implementation should be redacted?
    pub debug: PartToRedact,

    /// Which part of the value's [`Display`](std::fmt::Display) implementation should be redacted?
    pub display: PartToRedact,

    /// Which part of the value's [`Serialize`](serde::Serialize) implementation should be redacted?
    pub serialize: PartToRedact,

    /// Used when `Serialize` is called and the output is passed to `tracing`,
    /// instead of leaving through some API.
    ///
    /// When using `tracing`, a lot of times a struct's `Serialize` implementation is more useful than `Debug`.
    /// That's because `Debug` doesn't really have a defined syntax
    /// any post-processor of your tracing output could support.
    /// (For example, piping it into an OpenTelemetry stack)
    ///
    /// So, you might want to log your struct as JSON instead of `Debug`.
    /// This cases an issue because you want to redact your sensitive value's from `tracing`
    /// but you don't want to redact `Serialize` in general when communicating over a JSON API.
    ///
    /// This field in combination with [`Redacted::with_tracing_serialize`] form the escape hatch for this issue.
    pub tracing_serialize: PartToRedact,
}

/// What should be redacted?
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PartToRedact {
    /// Leave everything unredacted
    Nothing,

    /// The entire string
    Everything,

    /// Just the end of the string
    End {
        /// Start redaction after the first `after` bytes
        after: usize,
    },

    /// Just the middle of the string
    Middle {
        /// Start redaction after the first `after` bytes
        after: usize,

        /// Stop redaction before the last `before` bytes
        before: usize,
    },
}

impl PartToRedact {
    /// Redacts the part represented by `self` from `string`
    pub fn redact<'a>(&self, string: &'a str) -> Cow<'a, str> {
        match self {
            PartToRedact::Nothing => Cow::Borrowed(string),
            PartToRedact::Everything => Cow::Borrowed("-redacted-"),
            PartToRedact::End { after } => Cow::Owned(format!(
                "{}-redacted-",
                &string[..string.floor_char_boundary(*after)]
            )),
            PartToRedact::Middle { after, before } => Cow::Owned(format!(
                "{}-redacted-{}",
                &string[..string.floor_char_boundary(*after)],
                &string[string.ceil_char_boundary(*before)..]
            )),
        }
    }
}
