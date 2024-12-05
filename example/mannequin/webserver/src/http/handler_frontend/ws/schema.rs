//! The schema for the websocket connection

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Websocket messages that originate from the server
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "type")]
pub enum WsServerMsg {
    /// Internal use only.
    ///
    /// This variant is used to close the websocket connection
    #[serde(skip_serializing, skip_deserializing)]
    Close,
}

/// Websocket messages that originate from the client
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "type")]
pub enum WsClientMsg {}
