//! [`Module`] handling (graceful) shutdowns

use std::time::Duration;

use tokio::sync::SetOnce;
use tokio::sync::watch;
use tokio::time::timeout;
use tracing::warn;

use crate::InitError;
use crate::Module;
use crate::PostInitError;
use crate::PreInitError;

/// [`Module`] handling (graceful) shutdowns
///
/// *Choose the relevant TL;DR section by "what are you the author of".*
///
/// # TL;DR for application authors
///
/// Ignore this type. `Galvyn` will wrap its methods for you.
///
/// You might want to look at [`ShutdownSetup`] which is part of `GalvynSetup`.
///
/// # TL;DR for 3rd party lib authors
///
/// Ignore this type. `Galvyn` will wrap its methods for you.
///
/// # TL;DR for galvyn contrib authors
///
/// If you have tasks which should terminate before a graceful shutdown,
/// call [`Shutdown::block`] and store the returned guard in your task.
///
/// If your task runs forever,
/// use [`Shutdown::has_started`] or [`Shutdown::wait_for_started`] as a stop condition.
///
/// # Internals
///
/// At its core this type is a basic state machine:
/// `Start State -> Graceful Shutdown -> Shutdown Done`
///
/// The first transition can be triggered by calling [`Shutdown::start`] (`Galvyn::shutdown`).
/// The second transition will happen automatically if there are no "`blockers`" left (see below).
/// The state machine can also be forced into the last state by calling [`Shutdown::force_done`] (`Galvyn::kill`).
///
/// Each state transition can be listened for through [`Shutdown::wait_for_started`] and [`Shutdown::wait_for_done`].
/// Noteworthily, `Galvyn`'s `start` method terminates with `Shutdown::wait_for_done`.
///
/// ## Blockers
///
/// Any component of the app can block the second transition (at least the automatic one).
/// This is done by requesting a guard through [`Shutdown::block`].
/// Components (mostly `Module`s) which hold their guard for a long time (potentially indefinitely),
/// should use [`Shutdown::wait_for_started`] and drop their guard as soon as possible.
///
/// Blockers are implemented as a [`watch`] channel which never sends anything.
/// Instead, its `Drop` and close behavior is abused because it matches our needs almost exactly.
pub struct Shutdown {
    ongoing: SetOnce<Ongoing>,
    blockers: watch::Sender<()>,
    setup: ShutdownSetup,
}

/// `Setup` for [`Shutdown`]
#[derive(Debug)]
pub struct ShutdownSetup {
    /// Maximum time a graceful shutdown is allowed to take.
    ///
    /// Once a shutdown has been started (via [`Shutdown::start`]),
    /// the shutdown waits for all outstanding [`Shutdown::block`] guards to be dropped.
    /// If they are not dropped within `grace_period`,
    /// the shutdown is forced to completion as if [`Shutdown::force_done`] had been called.
    pub grace_period: Duration,
}

impl Default for ShutdownSetup {
    fn default() -> Self {
        Self {
            grace_period: Duration::from_secs(1),
        }
    }
}

impl Shutdown {
    /// Retrieves a guard which blocks a graceful shutdown until it is dropped
    ///
    /// (Wrapped as `Galvyn::block_shutdown`)
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
    /// (Wrapped as `Galvyn::shutdown_started`)
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
    /// (Wrapped as `Galvyn::shutdown`)
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn start(&self) {
        let _ = self.ongoing.set(Ongoing(SetOnce::new()));
    }

    /// Forces a shutdown to be done skipping a graceful shutdown period
    ///
    /// (Wrapped as `Galvyn::kill`)
    ///
    /// This method is **idempotent**.
    /// *(Calling it a second time has no effect.)*
    pub fn force_done(&self) {
        let _ = self.ongoing.set(Ongoing(SetOnce::new_with(Some(()))));
    }
}

/// Helper to give the nested `SetOnce` more semantic
///
/// This is the inner `SetOnce` in [`Shutdown`].
/// Its existence represents an ongoing shutdown.
/// Its internal "`bool`" marks the end of the shutdown.
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

impl Module for Shutdown {
    type Setup = ShutdownSetup;
    type PreInit = ShutdownPreInit;

    async fn pre_init(setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        Ok(ShutdownPreInit { setup })
    }

    type Dependencies = ();

    async fn init(
        pre_init: Self::PreInit,
        _dependencies: &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Shutdown {
            ongoing: SetOnce::new(),
            blockers: Default::default(),
            setup: pre_init.setup,
        })
    }

    async fn post_init(&'static self) -> Result<(), PostInitError> {
        tokio::spawn(async move {
            let ongoing = self.ongoing.wait().await;
            if let Err(_elapsed) = timeout(self.setup.grace_period, self.blockers.closed()).await {
                warn!(grace_period = ?self.setup.grace_period, "Graceful shutdown reached timeout. Forcing shutdown...");
            }
            ongoing.set_done();
        });
        Ok(())
    }
}

/// `PreInit` for [`Shutdown`]
pub struct ShutdownPreInit {
    setup: ShutdownSetup,
}
