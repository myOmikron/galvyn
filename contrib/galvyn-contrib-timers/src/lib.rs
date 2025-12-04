//! A galvyn [`Module`] which implements timers,
//!
//! think "cron" or "systemd-timers".
#![warn(missing_docs)]

use std::time::Duration;

use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PreInitError;

pub use crate::setup::TimersSetup;

mod setup;

/// TODO
pub struct Timers {}

/// Callback invoked by a timer
///
/// This is a trait alias for `FnMut` with the required multi-threading bounds.
/// Simply use a closure and let the compiler tell you if things don't work.
///
/// This trait can be implemented manually if a closure won't cut it.
pub trait TimerCallback: Send + Sync + 'static {
    /// Invokes this callback
    ///
    /// This function is executed in an async runtime (`tokio`).
    /// **It should not block!**
    /// Spawn your own (blocking) tokio task if you have to.
    fn call(&mut self);
}
impl<T: FnMut() + Send + Sync + 'static> TimerCallback for T {
    fn call(&mut self) {
        self()
    }
}

impl Timers {
    /// Schedules `callback` to run every `duration`
    pub fn schedule_every(&mut self, duration: Duration, mut callback: impl TimerCallback) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(duration);
            interval.tick().await;
            loop {
                interval.tick().await;
                callback.call();
            }
        });
    }
}

impl Module for Timers {
    type Setup = TimersSetup;
    type PreInit = PreInit;

    async fn pre_init(TimersSetup {}: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(PreInit {})
    }

    type Dependencies = ();

    async fn init(
        PreInit {}: Self::PreInit,
        (): &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Self {})
    }
}

/// [`Timers`]'s [`Module::PreInit`]
///
/// Not part of the public API!
pub struct PreInit {}
