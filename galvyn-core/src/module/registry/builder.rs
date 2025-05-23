use std::any::TypeId;
use std::any::type_name;
use std::error::Error;
use std::fmt;

use futures_concurrency::future::Join;
use futures_lite::future;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tracing::Instrument;
use tracing::debug;
use tracing::instrument;
use tracing::trace;
use tracing::trace_span;

use crate::module;
use crate::module::Module;
use crate::module::registry::DynModule;
use crate::module::registry::ModuleDependencies;
use crate::module::registry::Registry;
use crate::module::registry::module_set::OwnedModulesSet;

#[derive(Default)]
pub struct RegistryBuilder {
    modules: Vec<(TypeId, UninitModule)>,
}

impl RegistryBuilder {
    /// Constructs a new `RegistryBuilder`
    pub fn new() -> Self {
        Self::default()
    }

    fn contains_module(&self, type_id: TypeId) -> bool {
        self.modules.iter().find(|(id, _)| *id == type_id).is_some()
    }

    /// Adds a new module to the `RegistryBuilder`
    ///
    /// Calling this method twice with the same `T` is not an error but will only add it once.
    #[instrument(level = "trace", name = "RegistryBuilder::register_module", skip(self), fields(module.name = type_name::<T>()))]
    pub fn register_module<T: Module>(&mut self, setup: T::Setup) -> &mut Self {
        if self.contains_module(TypeId::of::<T>()) {
            panic!(
                "The module '{}' is being registered twice",
                type_name::<T>()
            );
        } else {
            <T::Dependencies as ModuleDependencies>::for_each(|dependency_id, dependency_name| {
                if !self.contains_module(dependency_id) {
                    panic!(
                        "Module '{}' depends on '{dependency_name}'. Please register it first.",
                        type_name::<T>(),
                    );
                }
            });

            self.modules.push((
                TypeId::of::<T>(),
                BoxDynFnOnce::new(move |()| {
                    tokio::spawn(async {
                        let pre_init = async move {
                            let result = T::pre_init(setup).await;
                            match &result {
                                Ok(_) => trace!("Finished pre init"),
                                Err(_) => trace!("Failed pre init"),
                            }
                            result
                        }
                        .instrument(trace_span!(
                            "Module::pre_init",
                            module.name = type_name::<T>()
                        ))
                        .await?;

                        Ok(BoxDynFnOnce::new(move |mut modules: OwnedModulesSet| {
                            Box::pin(async move {
                                let mut dependencies =
                                    <T::Dependencies as ModuleDependencies>::take(&mut modules);

                                let t = {
                                    let dependencies = &mut dependencies;
                                    async move {
                                        let result = T::init(pre_init, dependencies).await;
                                        match &result {
                                            Ok(_) => trace!("Finished init"),
                                            Err(_) => trace!("Failed init"),
                                        }
                                        result
                                    }
                                    .instrument(trace_span!(
                                        "Module::init",
                                        module.name = type_name::<T>()
                                    ))
                                    .await?
                                };

                                <T::Dependencies as ModuleDependencies>::put_back(
                                    dependencies,
                                    &mut modules,
                                );
                                modules.insert(t);
                                Ok(modules)
                            }) as future::Boxed<_>
                        }))
                    })
                }) as UninitModule,
            ));
            debug!(module.name = type_name::<T>(), "Registered module");
        }
        self
    }

    /// Initialized all registered modules
    ///
    /// and makes the registry available through [`Registry::global`].
    #[instrument(level = "trace", name = "RegistryBuilder::init", skip(self))]
    pub async fn init(&mut self) -> Result<(), InitError> {
        let pre_init_modules = process_join_results(
            self.modules
                .drain(..)
                .map(|(_, x)| x.call(()))
                .collect::<Vec<_>>()
                .join()
                .await,
        )
        .map_err(InitError::PreInit)?;

        let mut modules = OwnedModulesSet::new();
        for pre_init_module in pre_init_modules {
            modules = pre_init_module
                .call(modules)
                .await
                .map_err(InitError::Init)?;
        }

        let registry = {
            let global = Registry::raw_global();
            if global
                .set(Registry {
                    modules: modules.leak(),
                })
                .is_err()
            {
                panic!("The module registry has already been initialized once");
            }
            global
                .get()
                .unwrap_or_else(|| unreachable!("The OnceLock has just been set"))
        };

        process_join_results(
            registry
                .modules
                .iter()
                .map(|init_module| init_module.post_init())
                .collect::<Vec<_>>()
                .join()
                .await,
        )
        .map_err(InitError::PostInit)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum InitError {
    PreInit(Vec<module::PreInitError>),
    Init(module::InitError),
    PostInit(Vec<module::PostInitError>),
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let phase = match self {
            InitError::PreInit(_) => "pre-",
            InitError::Init(_) => "",
            InitError::PostInit(_) => "post-",
        };
        let (first, rest) = match self {
            InitError::PreInit(errors) => errors.split_first(),
            InitError::Init(error) => Some((error, [].as_slice())),
            InitError::PostInit(errors) => errors.split_first(),
        }
        .unwrap_or_else(|| unreachable!("Error lists should not be empty"));
        write!(f, "Error during module {phase}initialisation: {first}")?;
        if rest.is_empty() {
            write!(f, " (and {} more...)", rest.len())?;
        }
        Ok(())
    }
}
impl Error for InitError {}

/// An uninitialised module waiting to be pre-initialised
type UninitModule = BoxDynFnOnce<(), JoinHandle<Result<PreInitModule, module::PreInitError>>>;

/// A pre-initialised modules waiting to be initialised
type PreInitModule =
    BoxDynFnOnce<OwnedModulesSet, future::Boxed<Result<OwnedModulesSet, module::InitError>>>;

impl<M: Module> DynModule for M {
    fn post_init(&'static self) -> JoinHandle<Result<(), module::PostInitError>> {
        tokio::spawn(
            async move {
                let result = Module::post_init(self).await;
                match &result {
                    Ok(_) => trace!("Finished post init"),
                    Err(_) => trace!("Failed post init"),
                }
                result
            }
            .instrument(trace_span!(
                "Module::post_init",
                module.name = type_name::<Self>()
            )),
        )
    }
}

/// Helper mimicking a `Box<dyn FnOnce>` which doesn't exist because `FnOnce` isn't object safe.
struct BoxDynFnOnce<Arg, Ret>(Box<dyn FnMut(Arg) -> Ret + Send>);
impl<Arg: 'static, Ret: 'static> BoxDynFnOnce<Arg, Ret> {
    /// Constructs a new `BoxDynFnOnce`
    pub fn new(f: impl FnOnce(Arg) -> Ret + Send + 'static) -> Self {
        let mut f = Some(f);
        Self(Box::new(move |arg| {
            let f = f
                .take()
                .unwrap_or_else(|| unreachable!("The BoxDynFnOnce can only be called once"));
            f(arg)
        }))
    }

    /// Calls the contained `FnOnce`
    pub fn call(mut self, arg: Arg) -> Ret {
        (self.0)(arg)
    }
}

fn process_join_results<T, E: From<String>>(
    vec: Vec<Result<Result<T, E>, JoinError>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut ts = Vec::new();
    let mut errors = Vec::new();
    for join_result in vec {
        let result = join_result.unwrap_or_else(|join_error| {
            Err(E::from(
                join_error
                    .try_into_panic()
                    .map(|panic| {
                        format!(
                            "Module panicked: {}",
                            if let Some(string) = panic.downcast_ref::<String>() {
                                string.as_str()
                            } else if let Some(string) = panic.downcast_ref::<&'static str>() {
                                string
                            } else {
                                "Box<dyn Any>"
                            }
                        )
                    })
                    .unwrap_or_else(|join_error| format!("Couldn't join: {join_error}")),
            ))
        });

        match result {
            Ok(t) => ts.push(t),
            Err(error) => errors.push(error),
        }
    }

    if errors.is_empty() {
        Ok(ts)
    } else {
        Err(errors)
    }
}
