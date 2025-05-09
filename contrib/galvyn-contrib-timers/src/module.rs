use std::collections::HashMap;
use std::panic::Location;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

use crate::errors;
use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PreInitError;
use rorm::Database;

pub struct Timers {
    db: Database,

    /// Simple map tracking all keys passed to `add_timers` to detect duplicates early.
    ///
    /// The stored location is used to provide better error messages.
    keys: Mutex<HashMap<Arc<str>, &'static Location<'static>>>,
}

#[derive(Default, Debug)]
pub struct TimersSetup {
    private: (),
}

impl Timers {
    #[track_caller]
    pub fn add_timer(&'static self, key: &str) -> Result<TimerBuilder, errors::DuplicatedTimerKey> {
        let caller_location = Location::caller();

        let key = {
            let mut keys = self.keys.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(fst_location) = keys.get(key).copied() {
                return Err(errors::DuplicatedTimerKey {
                    key: key.to_string(),
                    locations: errors::DuplicatedTimerKeyLocations {
                        fst_location,
                        snd_location: caller_location,
                    },
                });
            }
            let arced_key = Arc::from(key);
            keys.insert(Arc::clone(&arced_key), caller_location);
            arced_key
        };

        Ok(TimerBuilder { timers: self, key })
    }
}

pub struct TimerBuilder {
    timers: &'static Timers,
    key: Arc<str>,
}

pub struct TimersPreInit {
    private: (),
}

impl Module for Timers {
    type Setup = TimersSetup;
    type PreInit = TimersPreInit;

    async fn pre_init(setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        let TimersSetup { private: () } = setup;
        Ok(TimersPreInit { private: () })
    }

    type Dependencies = (Database,);

    async fn init(
        pre_init: Self::PreInit,
        (db,): &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        let TimersPreInit { private: () } = pre_init;
        Ok(Self {
            db: db.clone(),
            keys: Mutex::new(Default::default()),
        })
    }
}
