use std::collections::HashMap;

use rorm::fields::types::Json;
use rorm::internal::field::Field;
use rorm::internal::field::FieldProxy;
use rorm::Model;
use rorm::Patch;
use serde_json::Value;
use time::OffsetDateTime;
use tower_sessions_rorm_store::SessionModel;

use crate::models::Session;

impl SessionModel for Session {
    fn get_expires_at_field() -> FieldProxy<impl Field<Type = OffsetDateTime, Model = Self>, Self> {
        Session::F.expires_at
    }

    fn get_data_field(
    ) -> FieldProxy<impl Field<Type = Json<HashMap<String, Value>>, Model = Self>, Self> {
        Session::F.data
    }

    fn get_insert_patch(
        id: String,
        expires_at: OffsetDateTime,
        data: Json<HashMap<String, Value>>,
    ) -> impl Patch<Model = Self> + Send + Sync + 'static {
        Self {
            id,
            expires_at,
            data,
            user: None,
        }
    }

    fn get_session_data(&self) -> (String, OffsetDateTime, Json<HashMap<String, Value>>) {
        (self.id.clone(), self.expires_at, self.data.clone())
    }
}
