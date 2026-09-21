use std::any::Any;

use tokio::sync::SetOnce;
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
///
/// # Experimental "multi-registry" feature
///
/// ## State of Implementation
///
/// This feature is really experimental.
/// It's currently just a random idea I had and wrote down.
/// It's not tested, thought through and missing any high-level API / docs.
///
/// ## Motivation
///
/// Galvyn's `Module` singletons are really convenient for the application authors
/// because they don't have to care about wiring their state.
///
/// However, it brings the classic drawbacks of global state.
/// For example, you can't have multiple instance of the "galvyn machinery" in the same process.
///
/// That's find for the intended primary usage of galvyn.
/// But there might be valid use cases – I'd say, mostly testing.
/// You might want to unit test the startup of a galvyn module without wiring its functions yourself,
/// or you might to spawn several server taking to each other in an integration test.
///
/// ## Constraints & Core Idea
///
/// A technic to support multiple registry while preserving the user API `T::global()` is well established.
/// It's some kind of thread local context functions and futures enter before running.
/// Think `tracing::Span::current()` or `tokio::runtime::Handle::current()`.
///
/// The tricky part is how to propagate the context to spawned threads or tasks.
/// The first of those examples, relies on the user propagating the context correctly and can be easy to get wrong.
/// The second example has the advantage of being the one spawning tasks and providing an alternative to spawning threads.
/// But even tokio provides some API for the user to manually propagate if it can't do it on its own.
///
/// **So, how should galvyn propagate the context?**
/// It doesn't. Let's piggyback on tokio.
///
/// Having yet another mechanism to worry about seems unacceptable.
/// When writing your application, you probably won't consider this feature at all.
/// So you won't write its required plumbing.
/// Until you need and discover it. At which point you don't want to revisit you entire >10k lines of server.
///
/// Galvyn runs (or is intended to run) in a tokio runtime.
/// Most users of galvyn will just have a single runtime – probably `#[tokio::main]`.
/// By attaching galvyn's state (which registry instance are we using) to tokio's context,
/// we can use the existing propagation method which mostly works and the user might already be familiar with.
///
/// The drawback of this design:
/// You have to create a new tokio runtime, if you want to create a new galvyn "instance".
/// This should be acceptable, for the described use case of testing.
///
/// tokio never intended for this and doesn't have any API for that.
/// However, it does provide an integer id for each created runtime.
/// We just associate those ids with our state in a global map.
///
/// (Their docs state it is not unique for the program's entire runtime.
/// However, at the moment of writing this, it is just an incrementing `u64`.
/// So it's unique enough. But might case issues in the future.)
pub struct Registry {
    modules: LeakedModuleSet,
}

trait DynModule: Any + Send + Sync + 'static {
    #[doc(hidden)]
    fn post_init(&'static self) -> (&'static str, JoinHandle<Result<(), module::PostInitError>>);
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

    #[track_caller]
    pub async fn global_wait() -> &'static Self {
        Self::raw_global().wait().await
    }

    #[track_caller]
    pub fn try_global() -> Option<&'static Self> {
        Self::raw_global().get()
    }

    #[track_caller]
    pub fn try_get<T: Module>(&self) -> Option<&T> {
        self.modules.get()
    }

    #[cfg(not(feature = "multi-registry"))]
    #[track_caller]
    fn raw_global() -> &'static SetOnce<Self> {
        static GLOBAL: SetOnce<Registry> = SetOnce::const_new();
        &GLOBAL
    }

    #[cfg(feature = "multi-registry")]
    #[track_caller]
    fn raw_global() -> &'static SetOnce<Self> {
        use std::collections::HashMap;
        use std::hash::BuildHasherDefault;
        use std::hash::DefaultHasher;
        use std::sync::Mutex;

        use tokio::runtime::Handle;
        use tokio::runtime::Id;

        static GLOBAL: Mutex<
            HashMap<Id, &'static SetOnce<Registry>, BuildHasherDefault<DefaultHasher>>,
        > = Mutex::new(HashMap::with_hasher(BuildHasherDefault::new()));
        let id = Handle::try_current()
            .expect("Can't access galvyn modules outside of a valid tokio runtime. This constraint is due to the multi-registry feature.").id();
        GLOBAL
            .lock()
            .expect("We won't panic after this")
            .entry(id)
            .or_insert_with(|| Box::leak(Box::new(SetOnce::new())))
    }
}
