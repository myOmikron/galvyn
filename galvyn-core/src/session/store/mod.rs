//! A session backend for managing session state.
//!
//! This crate provides the ability to use custom backends for session
//! management by implementing the [`SessionStore`] trait. This trait defines
//! the necessary operations for creating, saving, loading, and deleting session
//! records.
//!
//! # Implementing a Custom Store
//!
//! Below is an example of implementing a custom session store using an
//! in-memory [`HashMap`]. This example is for illustration purposes only; you
//! can use the provided [`MemoryStore`] directly without implementing it
//! yourself.
//!
//! ```rust
//! use std::{collections::HashMap, sync::Arc};
//!
//! use async_trait::async_trait;
//! use time::OffsetDateTime;
//! use tokio::sync::Mutex;
//! use tower_sessions_core::{
//!     session::{Id, Record},
//!     session_store, SessionStore,
//! };
//!
//! #[derive(Clone, Debug, Default)]
//! pub struct MemoryStore(Arc<Mutex<HashMap<Id, Record>>>);
//!
//! #[async_trait]
//! impl SessionStore for MemoryStore {
//!     async fn create(&self, record: &mut Record) -> session_store::Result<()> {
//!         let mut store_guard = self.0.lock().await;
//!         while store_guard.contains_key(&record.id) {
//!             // Session ID collision mitigation.
//!             record.id = Id::default();
//!         }
//!         store_guard.insert(record.id, record.clone());
//!         Ok(())
//!     }
//!
//!     async fn save(&self, record: &Record) -> session_store::Result<()> {
//!         self.0.lock().await.insert(record.id, record.clone());
//!         Ok(())
//!     }
//!
//!     async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
//!         Ok(self
//!             .0
//!             .lock()
//!             .await
//!             .get(session_id)
//!             .filter(|Record { expiry_date, .. }| is_active(*expiry_date))
//!             .cloned())
//!     }
//!
//!     async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
//!         self.0.lock().await.remove(session_id);
//!         Ok(())
//!     }
//! }
//!
//! fn is_active(expiry_date: OffsetDateTime) -> bool {
//!     expiry_date > OffsetDateTime::now_utc()
//! }
//! ```
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
//!
//! # ExpiredDeletion
//!
//! The [`ExpiredDeletion`] trait provides a method for deleting expired
//! sessions. Implementations can optionally provide a method for continuously
//! deleting expired sessions at a specified interval.

use std::fmt::Debug;

use async_trait::async_trait;

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
    async fn create(&self, session_record: &mut Record) -> StoreResult<()> {
        tracing::warn!(
            "The default implementation of `SessionStore::create` is being used, which relies on \
            `SessionStore::save`. To properly handle potential ID collisions, it is recommended that \
            stores implement their own version of `SessionStore::create`."
        );
        self.save(session_record).await?;
        Ok(())
    }

    /// Saves the provided session record to the store.
    ///
    /// This method is intended for updating the state of an existing session.
    async fn save(&self, session_record: &Record) -> StoreResult<()>;

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    async fn load(&self, session_id: &Id) -> StoreResult<Option<Record>>;

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    async fn delete(&self, session_id: &Id) -> StoreResult<()>;
}
