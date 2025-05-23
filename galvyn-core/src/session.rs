use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::str::FromStr;

use async_trait::async_trait;
use rorm::Database;
use rorm::Model;
use rorm::and;
use rorm::fields::types::Json;
use schemars::_serde_json::Value;
use thiserror::Error;
use tower_sessions::ExpiredDeletion;
use tower_sessions::Expiry;
pub use tower_sessions::Session;
use tower_sessions::SessionManagerLayer;
use tower_sessions::SessionStore;
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::time::OffsetDateTime;
pub use tower_sessions::session::Error;
use tower_sessions::session::Id;
use tower_sessions::session::Record;
use tower_sessions::session_store::Error as StoreError;
use tracing::debug;
use tracing::instrument;

use crate::Module;

pub fn layer() -> SessionManagerLayer<RormStore> {
    SessionManagerLayer::new(RormStore::new(Database::global().clone()))
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)))
        .with_same_site(SameSite::Lax)
}

#[derive(Model)]
pub struct GalvynSession {
    #[rorm(primary_key, max_length = 255)]
    id: String,
    expires_at: OffsetDateTime,
    data: Json<HashMap<String, Value>>,
}

/// The session store for rorm
pub struct RormStore {
    db: Database,
}

impl RormStore {
    /// Construct a new Store
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl Debug for RormStore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Galvyn rorm store")
    }
}

impl Clone for RormStore {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

#[async_trait]
impl SessionStore for RormStore {
    #[instrument(level = "trace")]
    async fn create(
        &self,
        session_record: &mut Record,
    ) -> tower_sessions::session_store::Result<()> {
        debug!("Creating new session");
        let mut tx = self
            .db
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
    async fn save(&self, session_record: &Record) -> tower_sessions::session_store::Result<()> {
        let Record {
            id,
            data,
            expiry_date,
        } = session_record;

        let mut tx = self
            .db
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
    async fn load(&self, session_id: &Id) -> tower_sessions::session_store::Result<Option<Record>> {
        debug!("Loading session");
        let db = &self.db;

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
    async fn delete(&self, session_id: &Id) -> tower_sessions::session_store::Result<()> {
        let db = &self.db;

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
    async fn delete_expired(&self) -> tower_sessions::session_store::Result<()> {
        let db = &self.db;

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
pub enum RormStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rorm::Error),
    #[error("Decoding of id failed: {0}")]
    DecodingFailed(#[from] base64::DecodeSliceError),
}

impl From<RormStoreError> for StoreError {
    fn from(value: RormStoreError) -> Self {
        Self::Backend(value.to_string())
    }
}
