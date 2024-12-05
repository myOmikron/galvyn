//! The handler for the websocket

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::response::Response;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::StreamExt;
use futures_util::SinkExt;
use futures_util::TryStreamExt;
use swaggapi::as_responses::simple_responses;
use swaggapi::as_responses::AsResponses;
use swaggapi::as_responses::SimpleResponse;
use swaggapi::get;
use swaggapi::internals::SchemaGenerator;
use swaggapi::re_exports::mime::APPLICATION_OCTET_STREAM;
use swaggapi::re_exports::openapiv3::Responses;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tower_sessions::Session;
use tracing::debug;
use tracing::trace;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::extractors::session_user::SessionUser;
use crate::http::handler_frontend::ws::schema::WsClientMsg;
use crate::http::handler_frontend::ws::schema::WsServerMsg;

struct WsResponse(Response);

impl IntoResponse for WsResponse {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

impl AsResponses for WsResponse {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        simple_responses([SimpleResponse {
            status_code: swaggapi::re_exports::openapiv3::StatusCode::Code(101),
            mime_type: APPLICATION_OCTET_STREAM,
            description: "Switching protocols".to_string(),
            media_type: None,
        }])
    }
}

#[get("/ws")]
pub async fn websocket(
    ws: WebSocketUpgrade,
    SessionUser { user, .. }: SessionUser,
    session: Session,
) -> WsResponse {
    let Some(id) = session.id() else {
        return WsResponse(
            ApiError::new_internal_server_error("Couldn't retrieve session".to_string())
                .into_response(),
        );
    };

    WsResponse(ws.on_upgrade(move |ws| async move {
        let (sender, receiver) = ws.split();

        let (tx_tx, tx_rx) = mpsc::channel(1);
        let (sender_tx, sender_rx) = mpsc::channel(1);

        let last_hb = Arc::new(Mutex::new(Instant::now()));

        let convert_handle = tokio::spawn(convert_to_send(sender_tx.clone(), tx_rx));
        let send_handle = tokio::spawn(handle_send(sender, sender_rx));
        let recv_handle = tokio::spawn(handle_recv(sender_tx.clone(), receiver));
        let heartbeat_handle = tokio::spawn(heartbeat(last_hb.clone(), sender_tx));

        if !GLOBAL.ws.register_ws(tx_tx, user.uuid, id).await {
            heartbeat_handle.abort();
            convert_handle.abort();
            send_handle.abort();
            recv_handle.abort();
        }
    }))
}

#[derive(Debug, Clone)]
enum SendInstruction {
    Message(Message),
    Close,
}

/// Convert incoming messages from GlobalWs to websocket Messages
async fn convert_to_send(
    sender_tx: mpsc::Sender<SendInstruction>,
    mut tx_rx: mpsc::Receiver<WsServerMsg>,
) {
    while let Some(msg) = tx_rx.recv().await {
        if let WsServerMsg::Close = msg {
            if let Err(err) = sender_tx.send(SendInstruction::Close).await {
                debug!("Error while sending: {err}");
                return;
            }
        }

        let Ok(serialized) = serde_json::to_string(&msg) else {
            continue;
        };

        if let Err(err) = sender_tx
            .send(SendInstruction::Message(Message::Text(serialized)))
            .await
        {
            debug!("Error while sending: {err}");
            return;
        }
    }
}

/// Send a heartbeat to the client
async fn heartbeat(last_hb: Arc<Mutex<Instant>>, sender_tx: mpsc::Sender<SendInstruction>) {
    const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

    loop {
        if Instant::now().duration_since(*last_hb.lock().await) > CLIENT_TIMEOUT
            && sender_tx.send(SendInstruction::Close).await.is_ok()
        {
            debug!("Closed websocket due to missing heartbeat responses");
            return;
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        if sender_tx
            .send(SendInstruction::Message(Message::Ping(vec![])))
            .await
            .is_err()
        {
            return;
        }
    }
}

/// Sends messages to the client
async fn handle_send(
    mut sender: SplitSink<WebSocket, Message>,
    mut tx_rx: mpsc::Receiver<SendInstruction>,
) {
    while let Some(instruction) = tx_rx.recv().await {
        match instruction {
            SendInstruction::Message(msg) => {
                if sender.send(msg).await.is_err() {
                    return;
                }
            }
            SendInstruction::Close => {
                if sender.close().await.is_err() {
                    debug!("Websocket already closed");
                }
                return;
            }
        }
    }
}

/// Handles messages from the client
async fn handle_recv(
    sender_tx: mpsc::Sender<SendInstruction>,
    mut receiver: SplitStream<WebSocket>,
) {
    while let Ok(Some(msg)) = receiver.try_next().await {
        match msg {
            Message::Text(data) => {
                let Ok(client_msg) = serde_json::from_str::<WsClientMsg>(&data) else {
                    debug!("Could not deserialize client message: {data}");
                    continue;
                };
            }
            Message::Ping(data) => {
                trace!("Received WS ping");
                if sender_tx
                    .send(SendInstruction::Message(Message::Pong(data)))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Message::Close(_) => {
                debug!("Client sent ws close");
                return;
            }
            _ => {}
        }
    }
}
