use std::{sync::Arc, time::Instant};

use actix_ws::{AggregatedMessage, AggregatedMessageStream};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::server::{MessageContent, ServerMessage};

#[derive(Deserialize)]
pub enum ClientMessage {
    Connect
}
#[derive(Clone)]
struct Chatter {
    username: String,
    profile_picture_key: String,
}
pub struct UserSession {
    pub session_id: Uuid,
    pub ws_server_tx: Sender<ServerMessage>,
    pub settings: Option<Settings>,
    pub current_call: Option<Uuid>,

    // if logged in
    pub user_details: Option<UserDetails>,
}
#[derive(Clone)]
pub struct UserDetails {
    pub chatter: Chatter,
    pub claims: Claims,
}
impl UserSession {
    pub fn new(ws_server_tx: Sender<ServerMessage>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            ws_server_tx,
            settings: None,
            current_call: None,

            // logged in
            user_details: None,
        }
    }
    pub async fn handle_message_from_client(
        &mut self, 
        stream: AggregatedMessage,
        mut session: actix_ws::Session,
        alive: Arc<tokio::sync::Mutex<Instant>>,
    ) {
        match stream {
            AggregatedMessage::Ping(bytes) => {
                if session.pong(&bytes).await.is_err() {
                    return;
                }
            }
            AggregatedMessage::Text(string) => {
                tracing::info!("Not accepting strings!");
            }
            AggregatedMessage::Close(reason) => {
                let _ = session.close(reason).await;
                tracing::info!("Got close, bailing");
                return;
            }
            AggregatedMessage::Pong(_) => {
                *alive.lock().await = Instant::now();
            }
            AggregatedMessage::Binary(msg) => {
                // match serde_json::from_slice::<session::ClientMessage>(&msg) {
                //     Ok(deserialized_message) => {
                //         ws_server_tx.send(ServerMessage {
                //             message_content: MessageContent::Connect(session.)
                //         })
                //     }
                //     Err(err) => {
                //         println!("Problem deserializing binary msg: {:#?}", err);
                //     }
                // }
            }
        }
    }
    pub async fn handle_message_from_server(&mut self, msg: ServerMessage) {
    }
}
#[derive(Clone)]
struct Claims {
    // id: ,
    exp: usize,
}
struct CallMeta {
    pub owner: Chatter,
    pub call_name: String,
    pub session_id: Uuid,
    pub connected_chatters: Vec<LiveChatter>,
    pub created_at: f64,
}
struct LiveChatter {
    session_id: Uuid,
    chatter: Chatter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub microphone_is_on: bool,
    pub camera_is_on: bool,
    pub sharing_screen: bool,
    pub global_muted: bool,
    pub deafened: bool,
}
