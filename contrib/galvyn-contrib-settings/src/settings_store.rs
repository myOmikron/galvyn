use std::collections::HashMap;
use std::collections::HashSet;

use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PostInitError;
use galvyn_core::PreInitError;
use galvyn_core::re_exports::rorm::Database;
use galvyn_core::re_exports::schemars::_serde_json::value::RawValue;
use galvyn_core::re_exports::serde::Serialize;
use galvyn_core::re_exports::serde::de::DeserializeOwned;
use galvyn_core::re_exports::serde_json;
use galvyn_core::re_exports::serde_json::value::to_raw_value;
use galvyn_core::re_exports::uuid::Uuid;
use rorm::fields::types::Json;
use rorm::fields::types::MaxStr;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::sync::watch;

use crate::model::GalvynSettings;

/// Galvyn [`Module`] storing settings on behalf of other modules.
///
/// It is recommended to use it through an [`ApplicationSettings`](crate::ApplicationSettings)
/// implementation.
pub struct SettingsStore {
    /// Stores all settings during galvyn's initialization to bulk the database access.
    ///
    /// This field is unused after initialization.
    ///
    /// 1. Populated with all existing settings from the database during [`SettingsStore::init`]
    /// 2. Extended with new settings from other modules during their `init`
    /// 3. New entries are written to database during [`SettingsStore::post_init`]
    entries: HashMap<SettingsKey, (EntryState, Box<RawValue>)>,

    /// Stores already registered keys to detect duplicates.
    ///
    /// This field is unused after initialization.
    registered_keys: HashSet<SettingsKey>,
}

/// The setup struct for the [`SettingsStore`] module
#[derive(Default, Debug)]
#[cfg_attr(doc, non_exhaustive)]
pub struct SettingsStoreSetup {}

impl SettingsStore {
    /// Registers a new settings key which stores a single value of type `T` in the database.
    ///
    /// This method will query the database for an existing value under this key.
    /// If there is no value yet, then this method will use `default` to generate and insert one.
    /// Either way it will return the current value.
    ///
    /// The `key` should uniquely identify your singleton of type `T` across server restarts.
    /// I.e. it should be some constant or a config value which is not changed after the initial start.
    ///
    /// The returned handle should be stored in [`ApplicationSettings`](crate::ApplicationSettings) or some other kind of [`Module`].
    pub fn register<T>(
        &mut self,
        key: &'static str,
        default: impl FnOnce() -> T,
    ) -> Result<SettingsHandle<T>, RegisterError>
    where
        T: Serialize + DeserializeOwned,
        T: Send + Sync + 'static,
    {
        let settings_key =
            SettingsKey(MaxStr::new(key.to_string()).map_err(|_| RegisterError::InvalidKey(key))?);

        if !self.registered_keys.insert(settings_key.clone()) {
            return Err(RegisterError::DuplicateKey(key));
        }

        let value = if let Some((_, raw_value)) = self.entries.get(&settings_key) {
            T::deserialize(&**raw_value).map_err(RegisterError::DeserializeCurrent)?
        } else {
            let value = default();
            self.entries.insert(
                settings_key.clone(),
                (
                    EntryState::New,
                    to_raw_value(&value).map_err(RegisterError::SerializeDefault)?,
                ),
            );
            value
        };

        let (sender, receiver) = watch::channel(value);

        Ok(SettingsHandle {
            key: settings_key,
            receiver,
            sender: Mutex::new(sender),
        })
    }
}

/// A registered settings singleton of type `T`
///
/// It provides a [`set`] method for updating the settings
/// and a several methods for retrieving it in various levels of abstraction.
pub struct SettingsHandle<T> {
    key: SettingsKey,
    receiver: watch::Receiver<T>,
    sender: Mutex<watch::Sender<T>>,
}

impl<T: Serialize> SettingsHandle<T> {
    /// Clones the current value.
    ///
    /// This method is preferred to [`borrow`] because it can't be used incorrectly
    /// and the cost of a `Clone` is usually neglectable.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.receiver.borrow().clone()
    }

    /// Borrows the current value.
    ///
    /// **Holding the reference will block updates!**
    /// It is advised to hold the reference for a short as possible.
    ///
    /// See [`watch::Receiver::borrow`] for more details.
    pub fn borrow(&self) -> watch::Ref<'_, T> {
        self.receiver.borrow()
    }

    /// Clones the underlying `watch::Receiver`.
    ///
    /// This enables the caller to wait for updates.
    pub fn watcher(&self) -> watch::Receiver<T> {
        self.receiver.clone()
    }

    /// Sets a new value.
    ///
    /// This method will write the new value to the database
    /// and notify everyone waiting through [`SettingsHandle::watcher`].
    pub async fn set(&self, value: T) -> Result<(), SetError> {
        let raw_value = to_raw_value(&value).map_err(SetError::Serialize)?;

        let sender = self.sender.lock().await;
        rorm::update(Database::global(), GalvynSettings)
            .set(GalvynSettings.value, Json(raw_value))
            .condition(GalvynSettings.key.equals(&*self.key.0))
            .await
            .map_err(SetError::Update)?;
        sender.send_replace(value);
        Ok(())
    }
}

/// Error returned by [`SettingsStore::register`]
#[derive(Error, Debug)]
pub enum RegisterError {
    /// The key is too long.
    ///
    /// A key must not be more than 255 bytes long.
    #[error("The settings key '{}' is too long.", 0.0)]
    InvalidKey(&'static str),

    /// The key has already been registered.
    ///
    /// This could have to cases:
    /// 1. Another module uses the same key -> try a different string
    /// 2. You registered your key twice -> check your code
    #[error("The settings key '{}' has already been used.", 0.0)]
    DuplicateKey(&'static str),

    /// The default value could not be serialized.
    ///
    /// This is necessary to write it to the database.
    ///
    /// Please check your type's [`Serialize`] implementation
    /// and the `default` closure passed to [`SettingsStore::register`].
    #[error("{0}")]
    SerializeDefault(serde_json::Error),

    /// Failed to deserialize the current value stored in the database.
    ///
    /// Most likely, your database has been written to by a different, incompatible version of your code.
    ///
    /// Did you change your settings type recently?
    /// Was this change backwards compatible?
    ///
    /// (Assuming your [`Serialize`] implementation matches your [`Deserialize`] implementation.)
    #[error("{0}")]
    DeserializeCurrent(serde_json::Error),
}

/// Error returned by [`SettingsHandle::set`]
#[derive(Error, Debug)]
pub enum SetError {
    /// The new value could not be serialized.
    ///
    /// This is necessary to write it to the database.
    ///
    /// Please check your type's [`Serialize`] implementation.
    #[error("{0}")]
    Serialize(serde_json::Error),

    /// The new value could not be written to the database.
    #[error("{0}")]
    Update(rorm::Error),
}

impl Module for SettingsStore {
    type Setup = SettingsStoreSetup;
    type PreInit = PreInit;

    async fn pre_init(setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(PreInit { setup })
    }

    type Dependencies = (Database,);

    async fn init(
        PreInit { setup: _ }: Self::PreInit,
        (db,): &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        let entries = rorm::query(&*db, GalvynSettings).all().await?;
        Ok(Self {
            entries: entries
                .into_iter()
                .map(|entry| {
                    (
                        SettingsKey(entry.key),
                        (EntryState::Existing, entry.value.0),
                    )
                })
                .collect(),
            registered_keys: HashSet::new(),
        })
    }

    async fn post_init(&'static self) -> Result<(), PostInitError> {
        rorm::insert(Database::global(), GalvynSettings)
            .bulk(self.entries.iter().filter_map(|(key, (state, value))| {
                matches!(state, EntryState::New).then_some(GalvynSettings {
                    uuid: Uuid::new_v4(),
                    key: key.0.clone(),
                    value: Json(value.clone()),
                })
            }))
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct SettingsKey(MaxStr<255>);

enum EntryState {
    New,
    Existing,
}

pub struct PreInit {
    #[expect(dead_code)]
    setup: SettingsStoreSetup,
}
