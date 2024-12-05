use crate::Module;
use rorm::fields::types::Json;
use rorm::internal::field::{Field, FieldProxy};
use rorm::{Database, Model, Patch};
use schemars::_serde_json::Value;
use std::collections::HashMap;
use tower_sessions::cookie::time::{Duration, OffsetDateTime};
use tower_sessions::cookie::SameSite;
pub use tower_sessions::Session;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_rorm_store::{RormStore, SessionModel};

pub fn layer() -> SessionManagerLayer<RormStore<GalvynSession>> {
    SessionManagerLayer::new(RormStore::<GalvynSession>::new(Database::global().clone()))
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

impl SessionModel for GalvynSession {
    fn get_expires_at_field() -> FieldProxy<impl Field<Type = OffsetDateTime, Model = Self>, Self> {
        GalvynSession::F.expires_at
    }

    fn get_data_field(
    ) -> FieldProxy<impl Field<Type = Json<HashMap<String, Value>>, Model = Self>, Self> {
        GalvynSession::F.data
    }

    fn get_insert_patch(
        id: String,
        expires_at: OffsetDateTime,
        data: Json<HashMap<String, Value>>,
    ) -> impl Patch<Model = Self> + Send + Sync + 'static {
        GalvynSession {
            id,
            data,
            expires_at,
        }
    }

    fn get_session_data(&self) -> (String, OffsetDateTime, Json<HashMap<String, Value>>) {
        (self.id.clone(), self.expires_at.clone(), self.data.clone())
    }
}
