use std::{sync::Arc, time::{Duration, Instant}};
use tokio::sync::{mpsc, oneshot};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, middleware::Logger, web};
use actix_ws::{AggregatedMessage, Session};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::{server::{MessageContent, ServerMessage}, session::UserSession};

mod server;
mod session;

async fn ws(req: HttpRequest, body: web::Payload, ws_server_tx: web::Data<Sender<server::ServerMessage>>) -> Result<HttpResponse, actix_web::Error> {
    let (response, session, stream) = actix_ws::handle(&req, body)?;
    // 128KB max size frames
    let mut stream = stream.max_frame_size(128 * 1024).aggregate_continuations();

    tracing::info!("Inserted session");

    let alive = Arc::new(tokio::sync::Mutex::new(Instant::now()));

    let mut session2 = session.clone();
    let alive2 = alive.clone();
    
    // remove Actix's Data's inner (arc) cause channels already reference count
    let (session_tx, mut session_rx) = mpsc::channel::<ServerMessage>(1024);
    let user_session = Arc::new(tokio::sync::Mutex::new(UserSession::new(Arc::into_inner(ws_server_tx.clone().into_inner()).unwrap())));

    // check if heartbeat died
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if session2.ping(b"").await.is_err() {
                break;
            }

            if Instant::now().duration_since(*alive2.lock().await) > Duration::from_secs(10) {
                let _ = session2.close(None).await;
                break;
            } 
        }
    });

    // one place to handle client messages (uses ws_server_tx),
    let user_session2 = user_session.clone();
    tokio::task::spawn_local(async move {
        while let Some(Ok(msg)) = stream.recv().await {
            user_session2.lock().await.handle_message_from_client(msg, session.clone(), alive.clone()).await;
        }
        let _ = session.close(None).await;
    });

    // one place to handle server messages (gives server a session_tx),
    let user_session3 = user_session.clone();
    tokio::task::spawn_local(async move {
        while let Some(msg) = session_rx.recv().await {
            user_session3.lock().await.handle_message_from_server(msg).await;
        }
    });
    let user_session4 = user_session.clone();
    tokio::spawn(async move {
        let (oneshot_tx, oneshot_rx) = oneshot::channel();
        let user_session4 = user_session4.lock().await;

        ws_server_tx.clone().send(ServerMessage {
            message_content: MessageContent::Connect {
                session_id: user_session4.session_id,
                session_tx: session_tx.clone(),
                settings: user_session4.settings.clone(),
                user_details: user_session4.user_details.clone()
            },
            responder: Some(oneshot_tx)
        }).await.unwrap();
        match oneshot_rx.await {
            Ok(response) => {
                println!("send connected message to server.");
            }
            Err(something) => {
                println!("Problem waiting for connect to server response: {:#?}", something);
            }
        }
    });
    

    // connect to ws server, send it ur transceiver
    

    tracing::info!("Spawned");

    Ok(response)
}


#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
        )
        .init();

    let (ws_server_tx, ws_server_rx) = tokio::sync::mpsc::channel::<ServerMessage>(1024);

    let mut ws_server = server::WsServer::new(ws_server_rx);

    tokio::spawn(async move {
        ws_server.handle_ws_messages().await;
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(ws_server_tx.clone()))
            .route("/ws", web::get().to(ws))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    Ok(())
}

