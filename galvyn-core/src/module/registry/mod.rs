use std::any::Any;
use std::sync::OnceLock;

use tokio::task::JoinHandle;

pub use self::dependencies::ModuleDependencies;
use crate::module;
use crate::module::Module;
use crate::module::registry::builder::RegistryBuilder;
use crate::module::registry::module_set::LeakedModuleSet;

pub mod builder;
mod dependencies;
mod module_set;

/// The registry stores [`Module`]s
///
/// is responsible for their initialization and grants access to them.
pub struct Registry {
    modules: LeakedModuleSet,
}

trait DynModule: Any + Send + Sync + 'static {
    #[doc(hidden)]
    fn post_init(&'static self) -> JoinHandle<Result<(), module::PostInitError>>;
}

impl Registry {
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder::new()
    }

    #[track_caller]
    pub fn global() -> &'static Self {
        let Some(global) = Self::raw_global().get() else {
            panic!("The global registry has not been initialized yet.");
        };
        global
    }

    pub fn try_global() -> Option<&'static Self> {
        Self::raw_global().get()
    }

    pub fn try_get<T: Module>(&self) -> Option<&T> {
        self.modules.get()
    }

    fn raw_global() -> &'static OnceLock<Self> {
        static GLOBAL: OnceLock<Registry> = OnceLock::new();
        &GLOBAL
    }
}
