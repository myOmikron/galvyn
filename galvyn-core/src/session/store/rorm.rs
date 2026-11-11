use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use async_trait::async_trait;
use rorm::Database;
use rorm::Model;
use rorm::and;
use rorm::db::transaction::TransactionError;
use rorm::fields::types::Json;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::debug;
use tracing::instrument;

use crate::Module;
use crate::session::config::SessionClock;
use crate::session::session::Id;
use crate::session::session::Record;
use crate::session::store::SessionStore;
use crate::session::store::StoreError;
use crate::session::store::StoreResult;

/// The session store for rorm
#[derive(Debug, Clone, Default)]
pub struct RormStore {
    db: RormStoreDb,
}

impl RormStore {
    /// Construct a new store with an explicit `db`
    ///
    /// Use `default` to access the database lazily through `::global()`.
    pub fn new(db: Database) -> Self {
        Self {
            db: RormStoreDb::Explicit(db),
        }
    }
}

#[derive(Model)]
pub struct GalvynSession {
    #[rorm(primary_key, max_length = 255)]
    id: String,
    expires_at: OffsetDateTime,
    data: Json<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Default)]
enum RormStoreDb {
    #[default]
    Module,
    Explicit(Database),
}

impl RormStoreDb {
    fn get(&self) -> &Database {
        match self {
            RormStoreDb::Module => Database::global(),
            RormStoreDb::Explicit(x) => x,
        }
    }
}

impl fmt::Debug for RormStoreDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Module => "Module",
            Self::Explicit(_) => "Explicit",
        })
    }
}

#[async_trait]
impl SessionStore for RormStore {
    #[instrument(level = "trace")]
    async fn create(&self, _clock: &SessionClock, session_record: &mut Record) -> StoreResult<()> {
        debug!("Creating new session");
        let mut tx = self
            .db
            .get()
            .start_transaction()
            .await
            .map_err(RormStoreError::from)?;
        loop {
            let existing = rorm::query(&mut tx, GalvynSession)
                .condition(GalvynSession.id.equals(session_record.id.to_string()))
                .optional()
                .await
                .map_err(RormStoreError::from)?;

            if existing.is_none() {
                rorm::insert(&mut tx, GalvynSession)
                    .return_nothing()
                    .single(&GalvynSession {
                        id: session_record.id.to_string(),
                        expires_at: session_record.expiry_date,
                        data: Json(session_record.data.clone()),
                    })
                    .await
                    .map_err(RormStoreError::from)?;

                break;
            }

            session_record.id = Id::default();
        }

        tx.commit().await.map_err(RormStoreError::from)?;

        Ok(())
    }

    #[instrument(level = "trace")]
    async fn save(&self, _clock: &SessionClock, session_record: &Record) -> StoreResult<()> {
        let Record {
            id,
            data,
            expiry_date,
        } = session_record;

        let mut tx = self
            .db
            .get()
            .start_transaction()
            .await
            .map_err(RormStoreError::from)?;

        let existing_session = rorm::query(&mut tx, GalvynSession)
            .condition(GalvynSession.id.equals(id.to_string()))
            .optional()
            .await
            .map_err(RormStoreError::from)?;

        if existing_session.is_some() {
            rorm::update(&mut tx, GalvynSession)
                .set(GalvynSession.expires_at, *expiry_date)
                .set(GalvynSession.data, Json(data.clone()))
                .condition(GalvynSession.id.equals(id.to_string()))
                .await
                .map_err(RormStoreError::from)?;
        } else {
            rorm::insert(&mut tx, GalvynSession)
                .single(&GalvynSession {
                    id: id.to_string(),
                    expires_at: *expiry_date,
                    data: Json(data.clone()),
                })
                .await
                .map_err(RormStoreError::from)?;
        }

        tx.commit().await.map_err(RormStoreError::from)?;

        Ok(())
    }

    #[instrument(level = "trace")]
    async fn load(&self, clock: &SessionClock, session_id: &Id) -> StoreResult<Option<Record>> {
        debug!("Loading session");
        let db = self.db.get();

        let session = rorm::query(db, GalvynSession)
            .condition(and!(
                GalvynSession.id.equals(session_id.to_string()),
                GalvynSession.expires_at.greater_than(clock.now_utc())
            ))
            .optional()
            .await
            .map_err(RormStoreError::from)?;

        Ok(match session {
            None => None,
            Some(session) => Some(Record {
                id: Id::from_str(session.id.as_str()).map_err(RormStoreError::from)?,
                data: session.data.into_inner(),
                expiry_date: session.expires_at,
            }),
        })
    }

    #[instrument(level = "trace")]
    async fn delete(&self, _clock: &SessionClock, session_id: &Id) -> StoreResult<()> {
        let db = self.db.get();

        rorm::delete(db, GalvynSession)
            .condition(GalvynSession.id.equals(session_id.to_string()))
            .await
            .map_err(RormStoreError::from)?;

        Ok(())
    }
}

/// Error type that is used in the [SessionStore] trait
#[derive(Debug, Error)]
#[allow(missing_docs)]
enum RormStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rorm::Error),
    #[error("Decoding of id failed: {0}")]
    DecodingFailed(#[from] base64::DecodeSliceError),
    // All the session code uses its own transaction internally and does not use hooks
    #[error("This is a bug in galvyn: {0}")]
    Hook(rorm::db::transaction::HookError),
}

impl From<TransactionError> for RormStoreError {
    fn from(value: TransactionError) -> Self {
        match value {
            TransactionError::Database(x) => Self::Database(x),
            TransactionError::Hook(x) => Self::Hook(x),
        }
    }
}
impl From<RormStoreError> for StoreError {
    fn from(value: RormStoreError) -> Self {
        Self::Backend(value.to_string())
    }
}
