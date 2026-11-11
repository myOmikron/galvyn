use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::session::config::SessionClock;
use crate::session::session::Id;
use crate::session::session::Record;
use crate::session::store::SessionStore;
use crate::session::store::StoreResult;

/// A session store that lives only in memory.
///
/// This is useful for testing but not recommended for real applications.
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<HashMap<Id, Record>>);

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create(&self, _clock: &SessionClock, record: &mut Record) -> StoreResult<()> {
        let mut store_guard = self.0.lock().await;
        while store_guard.contains_key(&record.id) {
            // Session ID collision mitigation.
            record.id = Id::default();
        }
        store_guard.insert(record.id, record.clone());
        Ok(())
    }

    async fn save(&self, _clock: &SessionClock, record: &Record) -> StoreResult<()> {
        self.0.lock().await.insert(record.id, record.clone());
        Ok(())
    }

    async fn load(&self, clock: &SessionClock, session_id: &Id) -> StoreResult<Option<Record>> {
        let now = clock.now_utc();
        Ok(self
            .0
            .lock()
            .await
            .get(session_id)
            .filter(|Record { expiry_date, .. }| *expiry_date > now)
            .cloned())
    }

    async fn delete(&self, _clock: &SessionClock, session_id: &Id) -> StoreResult<()> {
        self.0.lock().await.remove(session_id);
        Ok(())
    }
}
