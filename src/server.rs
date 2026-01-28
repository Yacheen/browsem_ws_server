use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::session::{ClientMessage, Settings, UserDetails};


pub struct WsServer {
    sessions: Arc<tokio::sync::Mutex<HashMap<Uuid, (Chatter, Sender<ClientMessage>)>>>,
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
                ServerMessage::Connect { chatter, session_tx } => {
                    tracing::info!("user trying to connect to sever");
                    let online_sessions: u32;
                    {
                        let mut sessions = self.sessions.lock().await;
                        let _ = sessions.insert(chatter.session_id, (chatter.clone(), session_tx.clone()));
                        online_sessions = sessions.len() as u32;
                    }
                    session_tx.send(ClientMessage::Connected {
                        session_id: chatter.session_id,
                        online_sessions,
                    }).await.unwrap();
                }
                ServerMessage::Disconnect { session_id, session_tx } => {
                    tracing::info!("user trying to disconnect to sever");
                    if let Some((_chatter, _)) = self.sessions.lock().await.remove(&session_id) {
                        session_tx.send(ClientMessage::Disconnected).await.unwrap();
                    }
                },
                ServerMessage::UpdateInfo { username, settings, session_id } => {
                    if let Some((chatter, _)) = self.sessions.lock().await.get_mut(&session_id) {
                        chatter.username = username;
                        chatter.settings = settings;
                    }
                }
                _ => (),
            }
        }
    }
}
#[derive(Debug)]
pub enum ServerMessage {
    Connect {
        chatter: Chatter,
        session_tx: Sender<ClientMessage>,
    },
    Disconnect {
        session_id: Uuid,
        session_tx: Sender<ClientMessage>,
    },
    UpdateInfo {
        username: String,
        settings: Option<Settings>,
        session_id: Uuid
    }
}
#[derive(Clone, Debug)]
pub struct Chatter {
    pub username: String,
    pub profile_picture_key: Option<String>,
    pub session_id: Uuid,
    pub settings: Option<Settings>,
}
