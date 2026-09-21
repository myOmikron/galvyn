use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::hash::Hasher;

use crate::module::Module;
use crate::module::registry::DynModule;

pub struct ModuleSet<M> {
    set: HashMap<TypeId, M, BuildXorHasher>,
}
pub type OwnedModulesSet = ModuleSet<Box<dyn DynModule>>;
pub type LeakedModuleSet = ModuleSet<&'static dyn DynModule>;

impl<M> Default for ModuleSet<M> {
    fn default() -> Self {
        Self::new()
    }
}
impl<M> ModuleSet<M> {
    pub fn new() -> Self {
        Self {
            set: HashMap::with_hasher(BuildXorHasher),
        }
    }
}
impl OwnedModulesSet {
    pub fn insert<T: Module>(&mut self, value: T) -> Option<Box<T>> {
        self.set
            .insert(TypeId::of::<T>(), Box::new(value))
            .map(downcast_box)
    }

    pub fn remove<T: Module>(&mut self) -> Option<Box<T>> {
        self.set.remove(&TypeId::of::<T>()).map(downcast_box)
    }

    pub fn leak(self) -> LeakedModuleSet {
        let mut modules = LeakedModuleSet::new();
        for (type_id, module) in self.set {
            modules.set.insert(type_id, Box::leak(module));
        }
        modules
    }
}
impl LeakedModuleSet {
    pub fn get<T: Module>(&self) -> Option<&'static T> {
        self.set.get(&TypeId::of::<T>()).copied().map(downcast_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static dyn DynModule> + use<'_> {
        self.set.values().copied()
    }
}

fn downcast_box<T: Module>(module: Box<dyn DynModule>) -> Box<T> {
    let module: Box<dyn Any> = module;
    module
        .downcast()
        .expect("The caller should pass the correct type and value")
}
fn downcast_ref<T: Module>(module: &dyn DynModule) -> &T {
    let module: &dyn Any = module;
    module
        .downcast_ref()
        .expect("The caller should pass the correct type and value")
}

struct BuildXorHasher;
impl BuildHasher for BuildXorHasher {
    type Hasher = XorHasher;

    fn build_hasher(&self) -> Self::Hasher {
        XorHasher { hash: 0 }
    }
}

struct XorHasher {
    hash: u64,
}
impl Hasher for XorHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut buffer = [0u8; 8];
        for chunk in bytes.chunks(8) {
            buffer.copy_from_slice(chunk);
            self.hash ^= u64::from_ne_bytes(buffer);
        }
    }
}
