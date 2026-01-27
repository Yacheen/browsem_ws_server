use std::sync::Arc;
use tokio::sync::oneshot;

use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::session::{Settings, UserDetails};


pub struct WsServer {
    sessions: Arc<tokio::sync::Mutex<Vec<String>>>,
    ws_server_rx: Receiver<ServerMessage>,
}
impl WsServer {
    pub fn new(ws_server_rx: Receiver<ServerMessage>) -> Self {
        WsServer {
            sessions: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            ws_server_rx: ws_server_rx,
        }
    }
    pub async fn handle_ws_messages(&mut self) {
        while let Some(message) = self.ws_server_rx.recv().await {
            match message.message_content {
                MessageContent::Connect { session_id, settings, session_tx, user_details } => {
                    println!("hello world");
                }
                _ => (),
            }
        }
    }
}
pub struct ServerMessage {
    pub message_content: MessageContent,
    pub responder: Option<oneshot::Sender<MessageContent>>,
}
pub enum MessageContent {
    Connect {
        session_id: Uuid,
        session_tx: Sender<ServerMessage>,
        settings: Option<Settings>,

        // logged in
        user_details: Option<UserDetails>,
    },
    // Connected {
    // }
}
