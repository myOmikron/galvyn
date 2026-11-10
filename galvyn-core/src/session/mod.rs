use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::str::FromStr;

use async_trait::async_trait;
use rorm::Database;
use rorm::Model;
use rorm::and;
use rorm::db::transaction::TransactionError;
use rorm::fields::types::Json;
use schemars::_serde_json::Value;
use thiserror::Error;
use time::Duration;
use time::OffsetDateTime;
use tower_cookies::cookie::SameSite;
use tracing::debug;
use tracing::instrument;

pub use self::service::SessionManager;
pub use self::service::SessionManagerLayer;
//
#[doc(inline)]
pub use self::{
    session::{Expiry, Session},
    session_store::{CachingSessionStore, ExpiredDeletion, SessionStore},
};
use crate::Module;
use crate::session::session::Id;
use crate::session::session::Record;

pub mod extract;
pub mod service;
pub mod session;
pub mod session_store;
//

pub fn layer() -> SessionManagerLayer<RormStore> {
    SessionManagerLayer::new(RormStore::default())
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)))
        .with_same_site(SameSite::Lax)
        .with_always_save(true)
}

#[derive(Model)]
pub struct GalvynSession {
    #[rorm(primary_key, max_length = 255)]
    id: String,
    expires_at: OffsetDateTime,
    data: Json<HashMap<String, Value>>,
}

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

impl Debug for RormStoreDb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Module => "Module",
            Self::Explicit(_) => "Explicit",
        })
    }
}

#[async_trait]
impl SessionStore for RormStore {
    #[instrument(level = "trace")]
    async fn create(&self, session_record: &mut Record) -> session_store::Result<()> {
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
    async fn save(&self, session_record: &Record) -> session_store::Result<()> {
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
    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        debug!("Loading session");
        let db = self.db.get();

        let session = rorm::query(db, GalvynSession)
            .condition(and!(
                GalvynSession.id.equals(session_id.to_string()),
                GalvynSession
                    .expires_at
                    .greater_than(OffsetDateTime::now_utc())
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
    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let db = self.db.get();

        rorm::delete(db, GalvynSession)
            .condition(GalvynSession.id.equals(session_id.to_string()))
            .await
            .map_err(RormStoreError::from)?;

        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for RormStore {
    #[instrument(level = "trace")]
    async fn delete_expired(&self) -> session_store::Result<()> {
        let db = self.db.get();

        rorm::delete(db, GalvynSession)
            .condition(
                GalvynSession
                    .expires_at
                    .less_than(OffsetDateTime::now_utc()),
            )
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
impl From<RormStoreError> for session_store::Error {
    fn from(value: RormStoreError) -> Self {
        Self::Backend(value.to_string())
    }
}
