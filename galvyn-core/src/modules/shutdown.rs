//! [`Module`] handling (graceful) shutdowns

use tokio::sync::SetOnce;
use tokio::sync::watch;

use crate::InitError;
use crate::Module;
use crate::PostInitError;
use crate::PreInitError;

/// [`Module`] handling (graceful) shutdowns
///
/// # Usage for _ authors
///
/// ## Application
///
/// Ignore this type. `Galvyn` will wrap it for you.
///
/// ## 3rd party lib
///
/// Ignore this type. `Galvyn` will wrap it for you.
///
/// ## galvyn contrib
///
/// If you have tasks which should terminate before a graceful shutdown,
/// call [`Shutdown::block`] and store the returned guard in your task.
///
/// If your task runs forever,
/// use [`Shutdown::has_started`] or [`Shutdown::wait_for_started`] as a stop condition.
///
/// # Internals
///
/// TODO
pub struct Shutdown {
    ongoing: SetOnce<Ongoing>,
    blockers: watch::Sender<()>,
}

struct Ongoing(SetOnce<()>);
impl Ongoing {
    fn is_done(&self) -> bool {
        self.0.initialized()
    }
    async fn wait_done(&self) {
        self.0.wait().await;
    }
    fn set_done(&self) {
        let _ = self.0.set(());
    }
}

impl Shutdown {
    /// Retrieves a guard which blocks a graceful shutdown until it is dropped
    pub fn block(&self) -> impl Drop + Send + Sync + 'static {
        self.blockers.subscribe()
    }

    /// Has a graceful shutdown been started?
    pub fn has_started(&self) -> bool {
        self.ongoing.initialized()
    }

    /// Is the shutdown completed?
    pub fn is_done(&self) -> bool {
        self.ongoing.get().is_some_and(Ongoing::is_done)
    }

    /// Waits until a graceful shutdown has been requested
    ///
    /// # Cancel Safety
    /// This method is cancel safe.
    pub async fn wait_for_started(&self) {
        self.ongoing.wait().await;
    }

    /// Waits until the shutdown has completed
    ///
    /// # Cancel Safety
    /// This method is cancel safe.
    pub async fn wait_for_done(&self) {
        let ongoing = self.ongoing.wait().await;
        ongoing.wait_done().await;
    }

    /// Starts a graceful shutdown
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn start(&self) {
        let _ = self.ongoing.set(Ongoing(SetOnce::new()));
    }

    /// Forces a shutdown to be done skipping a graceful shutdown period
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn force_done(&self) {
        let _ = self.ongoing.set(Ongoing(SetOnce::new_with(Some(()))));
    }
}

impl Module for Shutdown {
    type Setup = ShutdownSetup;
    type PreInit = ShutdownPreInit;

    async fn pre_init(_setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(ShutdownPreInit {})
    }

    type Dependencies = ();

    async fn init(
        _pre_init: Self::PreInit,
        _dependencies: &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Shutdown {
            ongoing: SetOnce::new(),
            blockers: Default::default(),
        })
    }

    async fn post_init(&'static self) -> Result<(), PostInitError> {
        tokio::spawn(async move {
            let ongoing = self.ongoing.wait().await;
            self.blockers.closed().await;
            ongoing.set_done();
        });
        Ok(())
    }
}

/// `Setup` for [`Shutdown`]
#[derive(Default, Debug)]
#[non_exhaustive]
pub enum ShutdownSetup {
    #[default]
    Default,
}

/// `PreInit` for [`Shutdown`]
pub struct ShutdownPreInit {}
