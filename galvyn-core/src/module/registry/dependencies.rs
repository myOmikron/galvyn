#![allow(private_interfaces)]

use crate::module::Module;
use crate::module::registry::module_set::OwnedModulesSet;
use crate::util_macros::impl_tuples;
use std::any::{TypeId, type_name};

/// A tuple of [`Module`]s which need to be initialized before another one which depends on them.
pub trait ModuleDependencies: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    fn for_each(func: impl FnMut(TypeId, &'static str));

    #[doc(hidden)]
    fn take(modules: &mut OwnedModulesSet) -> Self;

    #[doc(hidden)]
    fn put_back(self, modules: &mut OwnedModulesSet);
}

macro_rules! impl_module_dependencies {
    ($($T:ident),+) => {
        impl<$( $T: Module, )+> ModuleDependencies for ($( $T, )+) {
            fn for_each(mut func: impl FnMut(TypeId, &'static str)) {
                $(
                    func(TypeId::of::<$T>(), type_name::<$T>());
                )*
            }

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
    fn for_each(_func: impl FnMut(TypeId, &'static str)) {}

    fn take(_modules: &mut OwnedModulesSet) -> Self {
        ()
    }

    fn put_back(self, _modules: &mut OwnedModulesSet) {}
}
