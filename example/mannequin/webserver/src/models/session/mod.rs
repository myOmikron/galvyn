//! All session related models are defined here

use std::collections::HashMap;

use rorm::fields::types::Json;
use rorm::prelude::ForeignModel;
use rorm::Model;
use serde_json::Value;
use time::OffsetDateTime;

use crate::models::user::User;

mod impls;

/// The representation of a session in the database
#[derive(Model)]
pub struct Session {
    /// The primary key of a session
    #[rorm(primary_key, max_length = 255)]
    pub id: String,

    /// The point in time this session expires
    pub expires_at: OffsetDateTime,

    /// Additional session data
    pub data: Json<HashMap<String, Value>>,

    /// The relation to an optional user
    #[rorm(on_delete = "Cascade", on_update = "Cascade")]
    pub user: Option<ForeignModel<User>>,
}
