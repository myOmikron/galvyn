use galvyn_core::InitError;
use galvyn_core::Module;
use galvyn_core::PreInitError;
use galvyn_core::TryGlobalError;

use crate::RegisterError;
use crate::settings_store::SettingsStore;

/// A specialized [`Module`] intended to store your application's settings.
///
/// # "Settings" refresher
///
/// In galvyn's naming, **"settings"** refer to global configuration values
/// which can be changed at runtime (by an admin) and don't require a server restart.
/// They are (usually) stored in the database.
///
/// **"config"** values require a restart and access to the application's host server.
/// They are stored in files or environment variables.
///
/// **"setup"** values require a rebuild.
/// They are hard-coded in the source code or derived from config values.
///
/// # Example
///
/// ```rust
/// /// Settings required for my application
/// pub struct Settings {
///     /// Origin under which my webserver is exposed
///     pub origin: SettingsHandle<String>,
///
///     /// Settings for CORS (Cross-Origin-Resource-Sharing)
///     pub cors: SettingsHandle<Cors>,
/// }
///
/// /// Settings for CORS (Cross-Origin-Resource-Sharing)
/// #[derive(Default, Serialize, Deserialize)]
/// pub struct Cors {
///     pub allowed_origins: Vec<String>,
/// }
///
/// impl ApplicationSettings for Settings {
///     fn init(store: &mut SettingsStore) -> Result<Self, RegisterError> {
///         Ok(Self {
///             origin: store.register("Settings.origin", || "https://blog.example.com".to_string())?,
///             cors: store.register("Settings.cors", Default::default)?,
///         })
///     }
/// }
/// ```
///
/// # Limits
///
/// This trait is restricting by design.
///
/// It forces the application author using it to write one simple struct
/// containing its application's settings.
///
/// If its design is not fitting for your application,
/// then there is noting stopping your from writing a normal [`Module`] depending on [`SettingsStore`].
///
/// The same applies when developing a library module.
pub trait ApplicationSettings: Sized + Send + Sync + 'static {
    /// Initializes your application's settings
    ///
    /// This method should construct `Self` by calling [`SettingsStore::register`] once
    /// for each of its fields.
    fn init(store: &mut SettingsStore) -> Result<Self, RegisterError>;

    /// Gets the settings' global instance
    ///
    /// This method should be used after every modules' `init` ran.
    /// I.e. in a module's `post_init` or the applications operation after that.
    ///
    /// # Panics
    /// If the settings have not been initialized yet.
    fn global() -> &'static Self {
        Self::try_global().unwrap_or_else(|error| panic!("{error}"))
    }

    /// Gets the settings' global instance
    ///
    /// # Errors
    /// If the settings have not been initialized yet.
    fn try_global() -> Result<&'static Self, TryGlobalError> {
        Adapter::<Self>::try_global().map(|adapter| &adapter.0)
    }
}

/// Annoying glue code to convert a [`ApplicationSettings`] implementation
/// into a [`Module`] implementation.
///
/// This trait is hacky and subject to refactoring.
///
/// # Current usage
/// ```rust
/// # use galvyn_contrib_settings::ApplicationSettings;
/// # use galvyn_contrib_settings::SettingsStore;
/// # use galvyn_contrib_settings::RegisterError;
/// # use galvyn_contrib_settings::ApplicationSettingsExt;
/// # use galvyn_core::registry::builder::RegistryBuilder;
///
/// struct MySettings {}
/// impl ApplicationSettings for MySettings {
///     fn init(store: &mut SettingsStore) -> Result<Self, RegisterError> {
///         Ok(Self {})
///     }
/// }
///
/// # fn foo(builder: RegistryBuilder) {
/// builder.register_module::<<MySettings as ApplicationSettingsExt>::Module>(Default::default())
/// # }
/// ```
pub trait ApplicationSettingsExt: ApplicationSettings {
    /// `Self` wrapped to implement `Module`
    type Module: Module;
}
impl<T: ApplicationSettings> ApplicationSettingsExt for T {
    type Module = Adapter<T>;
}
pub struct Adapter<T: ApplicationSettings>(T);
impl<T: ApplicationSettings> Module for Adapter<T> {
    type Setup = Setup;
    type PreInit = PreInit;

    async fn pre_init(Setup {}: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(PreInit {})
    }

    type Dependencies = (SettingsStore,);

    async fn init(
        PreInit {}: Self::PreInit,
        (store,): &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Self(<T as ApplicationSettings>::init(store)?))
    }
}

#[derive(Default, Debug)]
pub struct Setup {}

pub struct PreInit {}
