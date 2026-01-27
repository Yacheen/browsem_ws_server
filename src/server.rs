use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::session::{Settings, UserDetails};


pub struct WsServer {
    sessions: Arc<tokio::sync::Mutex<HashMap<Uuid, Sender<ServerMessage>>>>,
    ws_server_rx: Receiver<ServerMessage>,
}
impl WsServer {
    pub fn new(ws_server_rx: Receiver<ServerMessage>) -> Self {
        WsServer {
            sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            ws_server_rx: ws_server_rx,
        }
    }
    pub async fn handle_ws_messages(&mut self) {
        while let Some(message) = self.ws_server_rx.recv().await {
            match message {
                ServerMessage::Connect { session_id, settings, session_tx, user_details } => {
                    tracing::info!("user trying to connect to sever");
                    let _ = self.sessions.lock().await.insert(session_id, session_tx);
                }
                ServerMessage::Disconnect { session_id, session_tx } => {
                    tracing::info!("user trying to disconnect to sever");
                    if let Some(sesh) = self.sessions.lock().await.remove(&session_id) {
                        sesh.send(ServerMessage::Disconnected).await.unwrap();
                    }
                }
                _ => (),
            }
        }
    }
}
pub enum ServerMessage {
    Connect {
        session_id: Uuid,
        session_tx: Sender<ServerMessage>,
        settings: Option<Settings>,

        // logged in
        user_details: Option<UserDetails>,
    },
    Disconnect {
        session_id: Uuid,
        session_tx: Sender<ServerMessage>,
    },
    Disconnected
}
