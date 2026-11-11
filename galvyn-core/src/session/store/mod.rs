//! A session backend for managing session state.
//!
//! This crate provides the ability to use custom backends for session
//! management by implementing the [`SessionStore`] trait. This trait defines
//! the necessary operations for creating, saving, loading, and deleting session
//! records.
//!
//! # Implementing a Custom Store
//!
//! Have a look at [`MemoryStore`].
//! It is a simple example for implementing a session store.
//!
//! # Session Store Trait
//!
//! The [`SessionStore`] trait defines the interface for session management.
//! Implementations must handle session creation, saving, loading, and deletion.
//!
//! # CachingSessionStore
//!
//! The [`CachingSessionStore`] provides a layered caching mechanism with a
//! cache as the frontend and a store as the backend. This can improve read
//! performance by reducing the need to access the backend store for frequently
//! accessed sessions.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::session::config::SessionClock;
use crate::session::config::SessionConfig;
use crate::session::session::Id;
use crate::session::session::Record;

pub mod cache;
pub mod memory;
pub mod rorm;

/// Stores must map any errors that might occur during their use to this type.
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("Encoding failed with: {0}")]
    Encode(String),

    #[error("Decoding failed with: {0}")]
    Decode(String),

    #[error("{0}")]
    Backend(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

/// Defines the interface for session management.
///
/// See [`session_store`](crate::session_store) for more details.
#[async_trait]
pub trait SessionStore: Debug + Send + Sync + 'static {
    /// Creates a new session in the store with the provided session record.
    ///
    /// Implementers must decide how to handle potential ID collisions. For
    /// example, they might generate a new unique ID or return `Error::Backend`.
    ///
    /// The record is given as an exclusive reference to allow modifications,
    /// such as assigning a new ID, during the creation process.
    async fn create(&self, clock: &SessionClock, session_record: &mut Record) -> StoreResult<()> {
        tracing::warn!(
            "The default implementation of `SessionStore::create` is being used, which relies on \
            `SessionStore::save`. To properly handle potential ID collisions, it is recommended that \
            stores implement their own version of `SessionStore::create`."
        );
        self.save(clock, session_record).await?;
        Ok(())
    }

    /// Saves the provided session record to the store.
    ///
    /// This method is intended for updating the state of an existing session.
    async fn save(&self, clock: &SessionClock, session_record: &Record) -> StoreResult<()>;

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    async fn load(&self, clock: &SessionClock, session_id: &Id) -> StoreResult<Option<Record>>;

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    async fn delete(&self, clock: &SessionClock, session_id: &Id) -> StoreResult<()>;
}

/// Bundles a [`SessionStore`] and a [`SessionConfig`] into a single entity
#[derive(Debug)]
pub(super) struct StoreAndConfig<T: ?Sized = dyn SessionStore> {
    pub config: SessionConfig,
    pub store: T,
}

impl StoreAndConfig {
    pub async fn create(&self, session_record: &mut Record) -> StoreResult<()> {
        self.store.create(&self.config.clock, session_record).await
    }

    pub async fn save(&self, session_record: &Record) -> StoreResult<()> {
        self.store.save(&self.config.clock, session_record).await
    }

    pub async fn load(&self, session_id: &Id) -> StoreResult<Option<Record>> {
        self.store.load(&self.config.clock, session_id).await
    }

    pub async fn delete(&self, session_id: &Id) -> StoreResult<()> {
        self.store.delete(&self.config.clock, session_id).await
    }
}
