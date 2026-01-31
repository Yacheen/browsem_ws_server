use std::{sync::Arc, time::Instant};

use actix_ws::{AggregatedMessage, AggregatedMessageStream, CloseReason};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::server::{Chatter, ServerMessage};

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    Connect,
    #[serde(rename_all = "camelCase")]
    Connected {
        online_sessions: u32,
        session_id: Uuid,
    },
    Disconnect,
    Disconnected,
    UpdateInfo {
        username: String,
        settings: Option<Settings>,
        current_url: Option<String>,
        current_origin: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    CreateChannel {
        channel_name: String,
        max_chatters: u16,
        url_origin: String,
        full_url: String,
    },
    #[serde(rename_all = "camelCase")]
    ErrorMessage {
       error_message: ErrorType, 
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorType {
    NoChannelName(String),
    ChannelNameTooLong(String),
}
pub struct UserSession {
    pub session_id: Uuid,
    pub username: String,
    pub ws_server_tx: Arc<Sender<ServerMessage>>,
    pub settings: Option<Settings>,
    pub current_call: Option<Uuid>,
    pub current_url: Option<String>,
    pub current_origin: Option<String>,

    // if logged in
    pub user_details: Option<UserDetails>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserDetails {
    pub profile_picture_key: String,
    pub claims: Claims,
}
impl UserSession {
    pub fn new(ws_server_tx: Arc<Sender<ServerMessage>>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            username: String::new(),
            ws_server_tx,

            // whenever popup opens, sends msg to server of what these currently are
            settings: None,
            current_call: None,
            current_origin: None,
            current_url: None,

            // logged in
            user_details: None,
        }
    }
    pub async fn handle_message_from_client(
        &mut self, 
        stream: AggregatedMessage,
        mut session: actix_ws::Session,
        alive: Arc<tokio::sync::Mutex<Instant>>,
        session_tx: Sender<ClientMessage>,
    ) {
        match stream {
            AggregatedMessage::Ping(bytes) => {
                if session.pong(&bytes).await.is_err() {
                    return;
                }
            }
            AggregatedMessage::Text(string) => {
                match serde_json::from_str::<ClientMessage>(&string) {
                    Ok(deserialized_message) => {
                        tracing::info!("Message from client: {:#?}", deserialized_message);
                        match deserialized_message {
                            ClientMessage::Disconnect => {
                                let _ = self.ws_server_tx.send(ServerMessage::Disconnect { session_id: self.session_id, session_tx: session_tx }).await;
                            }
                            ClientMessage::UpdateInfo { username, settings, current_origin, current_url } => {
                                self.settings = settings.clone();
                                self.username = username.clone();
                                self.current_origin = current_origin.clone();
                                self.current_url = current_url.clone();
                                let _ = self.ws_server_tx.send(ServerMessage::UpdateInfo { username, settings, session_id: self.session_id, current_url, current_origin }).await;

                            }
                            ClientMessage::CreateChannel { channel_name, max_chatters, url_origin, full_url } => {
                                if channel_name.len() > 0 {
                                    let _ = self.ws_server_tx.send(ServerMessage::CreateChannel {
                                        session_id: self.session_id,
                                        channel_name,
                                        full_url,
                                        url_origin,
                                        max_chatters,
                                        channel_owner: self.username.clone()
                                    }).await;
                                }
                                else if channel_name.len() > 50 {
                                    let msg = ClientMessage::ErrorMessage { error_message: ErrorType::ChannelNameTooLong("Channel name required".to_string())};
                                    send_client_message(msg, session).await;
                                }
                                else if channel_name.len() == 0 {
                                    let msg = ClientMessage::ErrorMessage { error_message: ErrorType::NoChannelName("Channel name may not exceed 50 characters.".to_string()) };
                                    send_client_message(msg, session).await;
                                }
                            }
                            _ => (),
                        }
                    }
                    Err(err) => {
                        tracing::error!("Problem deserializing string msg from client: {:#?}", err);
                    }
                }
            }
            AggregatedMessage::Close(reason) => {
                let _ = session.close(reason.clone()).await;
                tracing::info!("Got close, reason: {:#?}", reason);
                self.ws_server_tx.send(ServerMessage::Disconnect { session_id: self.session_id, session_tx }).await.unwrap();
                return;
            }
            AggregatedMessage::Pong(_) => {
                *alive.lock().await = Instant::now();
            }
            // send server msg
            AggregatedMessage::Binary(msg) => {
                tracing::warn!("Someones trying to send binary. Uh ohhhhhhhh.");
            }
        }
    }
    pub async fn handle_message_from_server(
        &mut self,
        msg: ClientMessage,
        mut session: actix_ws::Session,
    ) {
        tracing::info!("message from server: {:#?}", msg);
        send_client_message(msg, session).await;
    }
}
async fn send_client_message(msg: ClientMessage, mut session: actix_ws::Session) {
    match serde_json::to_string(&msg) {
        Ok(serialized_msg) => {
            if session.text(serialized_msg.clone()).await.is_err() {
                tracing::warn!("Err sending serialized msg to client: session closed.");
                tracing::warn!("Message: {:?}", serialized_msg);
            };
        }
        Err(err) => {
            tracing::error!("Problem serializing server msg for client: {:#?}", msg);
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    // id: ,
    pub exp: usize,
}
struct CallMeta {
    pub owner: Chatter,
    pub call_name: String,
    pub session_id: Uuid,
    pub connected_chatters: Vec<Chatter>,
    pub created_at: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub microphone_is_on: bool,
    pub camera_is_on: bool,
    pub sharing_screen: bool,
    pub deafened: bool,
}
