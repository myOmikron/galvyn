#![allow(private_interfaces)]

use crate::module::registry::builder::RegistryBuilder;
use crate::module::registry::module_set::OwnedModulesSet;
use crate::module::Module;
use crate::util_macros::impl_tuples;
use std::any::type_name;

/// A tuple of [`Module`]s which need to be initialized before another one which depends on them.
pub trait ModuleDependencies: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    fn register(builder: &mut RegistryBuilder);

    #[doc(hidden)]
    fn take(modules: &mut OwnedModulesSet) -> Self;

    #[doc(hidden)]
    fn put_back(self, modules: &mut OwnedModulesSet);
}

macro_rules! impl_module_dependencies {
    ($($T:ident),+) => {
        impl<$( $T: Module, )+> ModuleDependencies for ($( $T, )+) {
            fn register(builder: &mut RegistryBuilder) {$(
                builder.register_module::<$T>();
            )+}

            fn take(modules: &mut OwnedModulesSet) -> Self {
                ($(
                    *modules.remove::<$T>().unwrap_or_else(
                        || panic!("Module {} has not been initialised yet", type_name::<$T>())
                    ),
                )+)
            }

            fn put_back(self, modules: &mut OwnedModulesSet) {
                #[allow(non_snake_case)]
                let ($( $T, )+) = self;
                $(
                    modules.insert($T);
                )+
            }
        }
    };
}
impl_tuples!(impl_module_dependencies);
impl ModuleDependencies for () {
    fn register(_builder: &mut RegistryBuilder) {}

    fn take(_modules: &mut OwnedModulesSet) -> Self {
        ()
    }

    fn put_back(self, _modules: &mut OwnedModulesSet) {}
}
