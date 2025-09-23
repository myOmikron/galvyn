//! A standard way to store your application's settings
//!
//! # "Settings" refresher
//!
//! In galvyn's naming, **"settings"** refer to global configuration values
//! which can be changed at runtime (by an admin) and don't require a server restart.
//! They are (usually) stored in the database.
//!
//! **"config"** values require a restart and access to the application's host server.
//! They are stored in files or environment variables.
//!
//! **"setup"** values require a rebuild.
//! They are hard-coded in the source code or derived from config values.
//!
//! # Starting point
//! Create a struct and implement [`ApplicationSettings`] on it.
#![warn(missing_docs)]

pub use crate::application_settings::ApplicationSettings;
pub use crate::application_settings::ApplicationSettingsExt;
pub use crate::settings_store::RegisterError;
pub use crate::settings_store::SetError;
pub use crate::settings_store::SettingsHandle;
pub use crate::settings_store::SettingsStore;
pub use crate::settings_store::SettingsStoreSetup;

mod application_settings;
mod model;
mod settings_store;
