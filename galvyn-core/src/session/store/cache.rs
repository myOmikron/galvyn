use async_trait::async_trait;

use crate::session::session::Id;
use crate::session::session::Record;
use crate::session::store::SessionStore;
use crate::session::store::StoreResult;

/// Provides a layered caching mechanism with a cache as the frontend and a
/// store as the backend..
///
/// Contains both a cache, which acts as a frontend, and a store which acts as a
/// backend. Both cache and store implement `SessionStore`.
///
/// By using a cache, the cost of reads can be greatly reduced as once cached,
/// reads need only interact with the frontend, forgoing the cost of retrieving
/// the session record from the backend.
///
/// # Examples
///
/// ```rust,ignore
/// # tokio_test::block_on(async {
/// use tower_sessions::CachingSessionStore;
/// use tower_sessions_moka_store::MokaStore;
/// use tower_sessions_sqlx_store::{SqlitePool, SqliteStore};
/// let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
/// let sqlite_store = SqliteStore::new(pool);
/// let moka_store = MokaStore::new(Some(2_000));
/// let caching_store = CachingSessionStore::new(moka_store, sqlite_store);
/// # })
/// ```
#[derive(Debug, Clone)]
pub struct CachingSessionStore<Cache: SessionStore, Store: SessionStore> {
    cache: Cache,
    store: Store,
}

impl<Cache: SessionStore, Store: SessionStore> CachingSessionStore<Cache, Store> {
    /// Create a new `CachingSessionStore`.
    pub fn new(cache: Cache, store: Store) -> Self {
        Self { cache, store }
    }
}

#[async_trait]
impl<Cache, Store> SessionStore for CachingSessionStore<Cache, Store>
where
    Cache: SessionStore,
    Store: SessionStore,
{
    async fn create(&self, record: &mut Record) -> StoreResult<()> {
        self.store.create(record).await?;
        self.cache.create(record).await?;
        Ok(())
    }

    async fn save(&self, record: &Record) -> StoreResult<()> {
        let store_save_fut = self.store.save(record);
        let cache_save_fut = self.cache.save(record);

        futures_lite::future::try_zip(store_save_fut, cache_save_fut).await?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> StoreResult<Option<Record>> {
        match self.cache.load(session_id).await {
            // We found a session in the cache, so let's use it.
            Ok(Some(session_record)) => Ok(Some(session_record)),

            // We didn't find a session in the cache, so we'll try loading from the backend.
            //
            // When we find a session in the backend, we'll hydrate our cache with it.
            Ok(None) => {
                let session_record = self.store.load(session_id).await?;

                if let Some(ref session_record) = session_record {
                    self.cache.save(session_record).await?;
                }

                Ok(session_record)
            }

            // Some error occurred with our cache so we'll bubble this up.
            Err(err) => Err(err),
        }
    }

    async fn delete(&self, session_id: &Id) -> StoreResult<()> {
        let store_delete_fut = self.store.delete(session_id);
        let cache_delete_fut = self.cache.delete(session_id);

        futures_lite::future::try_zip(store_delete_fut, cache_delete_fut).await?;

        Ok(())
    }
}
