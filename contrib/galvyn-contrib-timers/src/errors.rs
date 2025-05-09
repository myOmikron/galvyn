//! Errors produced by this crate

use std::error::Error;
use std::fmt;
use std::panic::Location;

/// Error returned by [`Timers::add_timer`] if the `key` has already been added once.
///
/// Following this error's `source` will yield an enhanced error message.
#[derive(Debug)]
pub struct DuplicatedTimerKey {
    /// The key which has been passed multiple times to [`Timers::add_timer`]
    pub key: String,

    /// The locations of the calls to [`Timers::add_timer`]
    ///
    /// This field is returned as this error's `source`
    /// because it enhances the basic error message with further details.
    /// (Namely the locations of the problematic code)
    pub locations: DuplicatedTimerKeyLocations,
}

impl fmt::Display for DuplicatedTimerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The provided key '{}' is already associated with a timer.",
            &self.key
        )
    }
}

impl Error for DuplicatedTimerKey {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.locations)
    }
}

/// [`DuplicatedTimerKey`]'s "inner error"
///
/// `DuplicatedTimerKey` will return this type as its `source`
/// which will enhance the error message for callers which traverse error sources.
#[derive(Debug)]
pub struct DuplicatedTimerKeyLocations {
    /// The location of the first caller of [`Timers::add_timer`] to use the duplicated key
    pub fst_location: &'static Location<'static>,

    /// The location of the second caller of [`Timers::add_timer`] to use the duplicated key
    pub snd_location: &'static Location<'static>,
}

impl fmt::Display for DuplicatedTimerKeyLocations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The key was initially used by '{}:{}:{}' and then by '{}:{}:{}' again",
            self.fst_location.file(),
            self.fst_location.line(),
            self.fst_location.column(),
            self.snd_location.file(),
            self.snd_location.line(),
            self.snd_location.column(),
        )
    }
}

impl Error for DuplicatedTimerKeyLocations {}
