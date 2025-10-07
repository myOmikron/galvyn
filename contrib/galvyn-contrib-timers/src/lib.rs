//! A galvyn [`Module`] which implements timers,
//!
//! think "cron" or "systemd-timers".
#![warn(missing_docs)]

use std::convert::Infallible;
use std::future::pending;
use std::sync::RwLock;
use std::time::Duration;

use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PostInitError;
use galvyn_core::PreInitError;
use tokio::time;

pub use crate::setup::TimersSetup;
use crate::state::TimersState;

mod setup;
mod state;

/// TODO
pub struct Timers {
    state: RwLock<TimersState>,
}

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
    pub fn schedule_every(&mut self, duration: Duration, callback: impl TimerCallback) {
        self.state
            .write()
            .unwrap()
            .schedule_every(duration, callback);
    }

    async fn run(&'static self) -> Infallible {
        loop {
            let Some(next_run) = self.state.read().unwrap().next_time() else {
                return pending::<Infallible>().await;
            };
            time::sleep_until(next_run).await;
            self.state.write().unwrap().run(next_run);
        }
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
        Ok(Self {
            state: RwLock::new(TimersState::new()),
        })
    }

    async fn post_init(&'static self) -> Result<(), PostInitError> {
        tokio::spawn(self.run());
        Ok(())
    }
}

/// [`Timers`]'s [`Module::PreInit`]
///
/// Not part of the public API!
pub struct PreInit {}
