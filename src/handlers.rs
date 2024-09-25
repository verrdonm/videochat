use axum::{
    extract::{FromRef, ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, Path, State}, http::StatusCode, response::{IntoResponse, Response},
};
use axum_extra::{headers, TypedHeader};
use futures::stream::StreamExt;
use std::{net::SocketAddr, ops::ControlFlow, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::service::{P2pRoomService, Participant};

impl FromRef<AppState> for Arc<P2pRoomService> {
    fn from_ref(app_state: &AppState) -> Arc<P2pRoomService> {
        app_state.room_service.clone()
    }
}

#[derive(Deserialize, Serialize)]
pub struct JoinRoomRequest {
    name: String,
    sdp: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomResponse {
}


#[derive(Deserialize, Serialize)]
pub struct P2pConnectionResponse {
}

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Clone)]
pub struct AppState {
    // that holds some api specific state
    pub room_service: Arc<P2pRoomService>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all="lowercase")]
pub enum WsMessage {
    Offer{name: String, sdp: String},
    Answer{
        #[serde(rename = "senderName")]
        sender_name: String, 
        #[serde(rename = "recipientName")]
        recipient_name: String, 
        sdp: String
    },
    Candidate{
        sender_name: String, 
        candidate: String,
    },
}


// ConnectInfo(addr): ConnectInfo<SocketAddr>,
// user_agent: Option<TypedHeader<headers::UserAgent>>,
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(room_service): State<Arc<P2pRoomService>>, 
    Path((room, name)): Path<(String, String)>,
) -> impl IntoResponse {
    
    tracing::info!("ws connection for {room}, {name}");
    // let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    //     user_agent.to_string()
    // } else {
    //     String::from("Unknown browser")
    // };
    // println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, room, name, room_service))
}
//who: SocketAddr, 
/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, room: String, name: String, room_service: Arc<P2pRoomService>) {
    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        tracing::info!("Pinged {name}...");
    } else {
        tracing::info!("Could not send ping {name}!");
        return;
    }

    let (sender, mut receiver) = socket.split();

    tracing::info!("joining room: {name}");
    let _ = room_service.join_room(&room, Participant::new(name.clone(), None, sender)).await;

    tracing::info!("joined room, starting receive task: {name}");
    // This second task will receive messages from client and print them on server console
    let name_clone = name.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // print message and break if instructed to do so
            if process_message(msg, &room, &name_clone, room_service.clone()).await.is_break() {
                // TODO room_service.leave_room(&room, &name_clone).await;
                break;
            }
        }
        cnt
    });

    tracing::info!("started receive task select: {name}");
    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => tracing::info!("Received {b} messages"),
                Err(b) => tracing::info!("Error receiving messages {b:?}")
            }
        }
    }
    tracing::info!("passed select: {name}");
    // returning from the handler closes the websocket connection
    tracing::info!("Websocket context {name} destroyed");
}

/// who: SocketAddr, 
/// helper to print contents of messages to stdout. Has special treatment for Close.
async fn process_message(msg: Message, room: &str, name: &str, room_service: Arc<P2pRoomService>) -> ControlFlow<(), ()> {
    tracing::info!("processing message: {msg:?}");
    match msg {
        Message::Text(s) => {
            match serde_json::from_str::<WsMessage>(&s) {
                Ok(message) => {
                    match message {
                        WsMessage::Offer{name, sdp} => {
                            tracing::info!("{name} sent offer: {sdp}");
                            let _ = room_service.handle_sdp_offer(room, &name, sdp).await;
                            
                        }
                        WsMessage::Answer{sender_name, recipient_name, sdp} => {
                            tracing::info!("{sender_name} sent answer: {recipient_name}, {sdp}");
                            let _ = room_service.handle_sdp_answer(room, &sender_name, &recipient_name, sdp).await;
                        }
                        WsMessage::Candidate { sender_name, candidate } => {
                            let _ = room_service.handle_candidate(room, sender_name, candidate).await;
                        }
                    }
                }
                Err(e) => tracing::info!("failed to parse message as WsMessage: {e}"),
            }
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                tracing::info!(
                    ">>> {} sent close with code {} and reason `{}`",
                    name, cf.code, cf.reason
                );
            } else {
                tracing::info!(">>> {name} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }
        _m => {
            // println!("Unsupported message type: {m:?}");
        }
    }
    ControlFlow::Continue(())
}
