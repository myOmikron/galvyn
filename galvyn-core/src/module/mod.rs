//! Types and traits to declare modules
//!
//! A module is a singleton which exists for the entire duration of the application.

use std::any::type_name;
use std::error::Error;
use std::fmt;
use std::future::Future;

use thiserror::Error;

pub use crate::module::impls::database::DatabaseSetup;
use crate::module::registry::ModuleDependencies;
use crate::module::registry::Registry;

mod impls;
pub mod registry;

/// A module is a globally available singleton which exists for the entire duration of the application.
///
/// # Initialization
///
/// Modules are initialized in three steps.
/// If any module returns an error in any of those steps,
/// the entire application will be prevented from starting.
///
/// ## pre init
///
/// Every modules' `pre_init` function is run concurrently.
///
/// In this step a module may not interact with any other module
/// but is free to load configuration or other data
/// entirely managed by the module itself.
///
/// The `pre_init` function may return data (`Self::PreInit`)
/// which is then passed to the actual `init` function.
///
/// ## init
///
/// Every modules' `init` function is run sequentially in the order defined by the application author.
///
/// This step should perform the module's main initialization
/// and returns the module singleton which is stored in globally.
///
/// TODO: dependencies
///
/// ## post init
///
/// Every modules' `post_init` function is run concurrently.
///
/// At this point every module has been initialized and is available globally.
///
/// Modules which others depend on have been made aware of their dependents and may run some finishing initialization code.
pub trait Module: Sized + Send + Sync + 'static {
    /// A type which is constructed by an application author to declare how this module should configure itself.
    ///
    /// This type should be constructed in a semi-constant fashion by the application author.
    /// It should not be constructed from configuration
    /// (things a sysadmin would tweak during the deployment).
    /// Instead, it should declare how the sysadmin is able to configure the module.
    ///
    /// If your module doesn't need to support different setups then `()` would be a good default.
    type Setup: fmt::Debug + Default + Sized + Send + Sync + 'static;

    /// Arbitrary data passed from the `pre_init` step to `init`
    ///
    /// No code external to a module's implementation should depend on this type.
    /// I.e. calling code should simply pass on `PreInit` from `pre_init` to `init`.
    ///
    /// If your module doesn't need any `pre_init` logic then `()` would be a good default.
    type PreInit: Sized + Send + Sync + 'static;

    /// Pre initialization run concurrently with all other modules'
    ///
    /// (see [Module Pre Init](Module#pre_init))
    ///
    /// If your module doesn't need any `pre_init` logic then `async { Ok(()) }` would be a good default.
    fn pre_init(
        setup: Self::Setup,
    ) -> impl Future<Output = Result<Self::PreInit, PreInitError>> + Send;

    /// A tuple of [`Module`]s which need to be initialized before this one.
    type Dependencies: ModuleDependencies;

    /// The main initialization of the module
    ///
    /// (see [Module Init](Module#init))
    fn init(
        pre_init: Self::PreInit,
        dependencies: &mut Self::Dependencies,
    ) -> impl Future<Output = Result<Self, InitError>> + Send;

    /// Post initialization run concurrently with all other modules'
    ///
    /// (see [Module Post Init](Module#post_init))
    fn post_init(&'static self) -> impl Future<Output = Result<(), PostInitError>> + Send {
        async { Ok(()) }
    }

    /// Gets the module's global instance
    ///
    /// This method should be used after every modules' `init` ran.
    /// I.e. in a module's `post_init` or the applications operation after that.
    ///
    /// # Panics
    /// If the module has not been initialized yet.
    fn global() -> &'static Self {
        Self::try_global().unwrap_or_else(|error| panic!("{error}"))
    }

    /// Gets the module's global instance
    ///
    /// # Errors
    /// If the module has not been initialized yet.
    fn try_global() -> Result<&'static Self, TryGlobalError> {
        Registry::try_global()
            .ok_or(TryGlobalError::Registry)?
            .try_get()
            .ok_or_else(|| TryGlobalError::Module {
                module_type: type_name::<Self>(),
            })
    }
}

pub type PreInitError = Box<dyn Error + Send + Sync + 'static>;
pub type InitError = Box<dyn Error + Send + Sync + 'static>;
pub type PostInitError = Box<dyn Error + Send + Sync + 'static>;

/// Error returned by [`Module::try_global`]
#[derive(Error, Debug)]
pub enum TryGlobalError {
    /// The module `Registry` has not been initialized yet.
    ///
    /// This might be caused by an invalid `Module` or `main` implementation
    /// because no code should try to access any module globally before the `Registry` has been initialized.
    #[error("the module registry has not been initialized yet")]
    Registry,

    /// The requested `Module` was not found in the registry
    ///
    /// This might be caused by an invalid `Module` which forgot to mention one of its dependencies
    /// or the application author who forgot to register all modules used by the application.
    #[error("the module '{module_type}' has not been registered")]
    Module {
        /// [`type_name`] of the requested `Module`
        module_type: &'static str,
    },
}
