//! The global websocket manager

use std::collections::HashMap;

use tokio::sync::mpsc;
use tower_sessions::session::Id;
use tracing::debug;
use tracing::error;
use uuid::Uuid;

use crate::http::handler_frontend::ws::schema::WsServerMsg;

/// The global websocket manager
pub struct GlobalWs {
    tx: mpsc::Sender<WsMessage>,
}

impl GlobalWs {
    /// Create a new instance of the global websocket manager
    ///
    /// A new task will be spawned that runs the manager
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);

        tokio::spawn(run_ws_manager(rx));

        Self { tx }
    }

    /// Close a specific websocket session of a user
    pub async fn close_session(&self, user: Uuid, session: Id) {
        if let Err(err) = self.tx.send(WsMessage::SessionClose((user, session))).await {
            error!("Could not send to GlobalWs: {err}");
        }
    }

    /// Close the websocket session for a user
    pub async fn close_user(&self, user: Uuid) {
        if let Err(err) = self.tx.send(WsMessage::UserClose(user)).await {
            error!("Could not send to GlobalWs: {err}");
        }
    }

    /// Send a message to the user.
    ///
    /// Note that the message will be sent to every session of the user
    ///
    /// If you don't want this behavior, use [GlobalWs::send_to_session]
    pub async fn send_to_user(&self, user: Uuid, message: WsServerMsg) {
        if let Err(err) = self.tx.send(WsMessage::UserMessage((user, message))).await {
            error!("Could not send to GlobalWs: {err}");
        }
    }

    /// Send a message to a session
    ///
    /// The message will only be sent to the specified session.
    /// If you want to send to a user, regardless of session, use [GlobalWs::send_to_user].
    pub async fn send_to_session(&self, user: Uuid, session: Id, message: WsServerMsg) {
        if let Err(err) = self
            .tx
            .send(WsMessage::SessionMessage((user, session, message)))
            .await
        {
            error!("Could not send to GlobalWs: {err}");
        }
    }

    /// Register a new websocket connection
    pub async fn register_ws(
        &self,
        sender: mpsc::Sender<WsServerMsg>,
        user: Uuid,
        session: Id,
    ) -> bool {
        if self
            .tx
            .send(WsMessage::NewClient((sender, user, session)))
            .await
            .is_err()
        {
            return false;
        }

        true
    }
}

async fn run_ws_manager(mut rx: mpsc::Receiver<WsMessage>) {
    let mut clients: HashMap<Uuid, HashMap<Id, mpsc::Sender<WsServerMsg>>> = HashMap::new();

    while let Some(ws_msg) = rx.recv().await {
        match ws_msg {
            WsMessage::NewClient((sender, user, session)) => {
                clients
                    .entry(user)
                    .and_modify(|sessions| {
                        sessions.insert(session, sender.clone());
                    })
                    .or_insert(HashMap::from([(session, sender)]));
            }

            WsMessage::SessionMessage((user, session, msg)) => {
                let Some(sessions) = clients.get_mut(&user) else {
                    debug!("User {user} was not found in GlobalWs");
                    continue;
                };

                let Some(sender) = sessions.get(&session) else {
                    continue;
                };

                if sender.send(msg).await.is_err() {
                    sessions.remove(&session);
                }
            }

            WsMessage::UserMessage((user, msg)) => {
                let Some(sessions) = clients.get_mut(&user) else {
                    debug!("User {user} was not found in GlobalWs");
                    continue;
                };

                let mut failed = vec![];
                for (id, sender) in sessions.iter() {
                    if sender.send(msg.clone()).await.is_err() {
                        debug!("Sending to websocket failed");
                        failed.push(*id);
                    }
                }

                if !failed.is_empty() {
                    debug!("Removing closed websockets from GlobalWs");
                    for id in failed {
                        sessions.remove(&id);
                    }
                }
            }
            WsMessage::SessionClose((user, session)) => {
                if let Some(sessions) = clients.get_mut(&user) {
                    if let Some(sender) = sessions.remove(&session) {
                        let _ = sender.send(WsServerMsg::Close).await;
                    }
                }
            }
            WsMessage::UserClose(user) => {
                if let Some(sessions) = clients.remove(&user) {
                    for (_, sender) in sessions {
                        let _ = sender.send(WsServerMsg::Close).await;
                    }
                }
            }
        }
    }
}

enum WsMessage {
    NewClient((mpsc::Sender<WsServerMsg>, Uuid, Id)),
    SessionMessage((Uuid, Id, WsServerMsg)),
    UserMessage((Uuid, WsServerMsg)),
    SessionClose((Uuid, Id)),
    UserClose(Uuid),
}

impl Default for GlobalWs {
    fn default() -> Self {
        Self::new()
    }
}
