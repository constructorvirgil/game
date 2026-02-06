use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

mod protocol;
mod rooms;

use protocol::{ClientMessage, ServerMessage};
use rooms::{display_name_for_user, PlayerConn, RoomError, RoomManager};

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<RoomManager>>,
}

#[derive(Clone)]
struct SessionBinding {
    room_id: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let state = AppState {
        rooms: Arc::new(Mutex::new(RoomManager::new())),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "ok" }))
        .nest_service(
            "/assets",
            ServeDir::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")),
        )
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any));

    let addr = SocketAddr::from(([0, 0, 0, 0], 33030));
    info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let user_id = rand::thread_rng().gen::<u64>();
    let user_name = display_name_for_user(user_id);
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let _ = tx.send(ServerMessage::Welcome {
        user_id,
        user_name: user_name.clone(),
    });
    send_room_list(&state, &tx).await;

    let mut current_room: Option<SessionBinding> = None;

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client) => {
                    if let Err(err) = handle_client_message(
                        &state,
                        user_id,
                        &user_name,
                        &tx,
                        &mut current_room,
                        client,
                    )
                    .await
                    {
                        let _ = tx.send(ServerMessage::Error {
                            message: format!("{:?}", err),
                        });
                    }
                }
                Err(_) => {
                    let _ = tx.send(ServerMessage::Error {
                        message: "invalid message".to_string(),
                    });
                }
            }
        }
    }

    leave_room_if_needed(&state, &mut current_room, user_id).await;
}

async fn handle_client_message(
    state: &AppState,
    user_id: u64,
    user_name: &str,
    tx: &mpsc::UnboundedSender<ServerMessage>,
    current_room: &mut Option<SessionBinding>,
    client: ClientMessage,
) -> Result<(), RoomError> {
    match client {
        ClientMessage::Ping => {
            let _ = tx.send(ServerMessage::Pong);
        }
        ClientMessage::ListRooms => {
            send_room_list(state, tx).await;
        }
        ClientMessage::CreateRoom => {
            leave_room_if_needed(state, current_room, user_id).await;

            let (room_id, player_count, started) = {
                let mut rooms = state.rooms.lock().await;
                let room_id = rooms.create_room();
                rooms.join_room(
                    &room_id,
                    PlayerConn {
                        id: user_id,
                        tx: Some(tx.clone()),
                    },
                )?;
                let _ = rooms.start_if_ready(&room_id, rand::random());
                let player_count = rooms.room_player_count(&room_id).unwrap_or(1);
                let started = rooms.room_started(&room_id).unwrap_or(false);
                (room_id, player_count, started)
            };

            *current_room = Some(SessionBinding {
                room_id: room_id.clone(),
            });
            let _ = tx.send(ServerMessage::RoomCreated {
                room_id: room_id.clone(),
            });
            let _ = tx.send(ServerMessage::Joined {
                room_id: room_id.clone(),
                you: user_id,
                you_name: user_name.to_string(),
                player_count,
                started,
            });

            broadcast_room_state(state, &room_id).await;
            send_room_list(state, tx).await;
        }
        ClientMessage::JoinRoom { room_id } => {
            leave_room_if_needed(state, current_room, user_id).await;
            let normalized_room = room_id.trim().to_uppercase();

            let (player_count, started) = {
                let mut rooms = state.rooms.lock().await;
                rooms.join_room(
                    &normalized_room,
                    PlayerConn {
                        id: user_id,
                        tx: Some(tx.clone()),
                    },
                )?;
                let _ = rooms.start_if_ready(&normalized_room, rand::random());
                let player_count = rooms.room_player_count(&normalized_room).unwrap_or(0);
                let started = rooms.room_started(&normalized_room).unwrap_or(false);
                (player_count, started)
            };

            *current_room = Some(SessionBinding {
                room_id: normalized_room.clone(),
            });
            let _ = tx.send(ServerMessage::Joined {
                room_id: normalized_room.clone(),
                you: user_id,
                you_name: user_name.to_string(),
                player_count,
                started,
            });

            broadcast_room_state(state, &normalized_room).await;
            send_room_list(state, tx).await;
        }
        ClientMessage::Play { cards } => {
            let room_id = current_room
                .as_ref()
                .map(|binding| binding.room_id.clone())
                .ok_or(RoomError::NotFound)?;
            let card_objs: Vec<game_core::Card> = cards
                .iter()
                .filter_map(|code| game_core::Card::from_code(code))
                .collect();
            if card_objs.len() != cards.len() {
                let _ = tx.send(ServerMessage::PlayRejected {
                    reason: "invalid card code".to_string(),
                });
                return Ok(());
            }
            let winner = {
                let mut rooms = state.rooms.lock().await;
                rooms.apply_play(&room_id, user_id, card_objs)?
            };
            broadcast_room_state(state, &room_id).await;
            if let Some(winner_id) = winner {
                broadcast_game_over(state, &room_id, winner_id).await;
            }
        }
        ClientMessage::Pass => {
            let room_id = current_room
                .as_ref()
                .map(|binding| binding.room_id.clone())
                .ok_or(RoomError::NotFound)?;
            {
                let mut rooms = state.rooms.lock().await;
                rooms.pass_turn(&room_id, user_id)?;
            }
            broadcast_room_state(state, &room_id).await;
        }
        ClientMessage::RestartGame => {
            let room_id = current_room
                .as_ref()
                .map(|binding| binding.room_id.clone())
                .ok_or(RoomError::NotFound)?;
            {
                let mut rooms = state.rooms.lock().await;
                rooms.restart_game(&room_id, user_id, rand::random())?;
            }
            broadcast_room_state(state, &room_id).await;
            broadcast_game_restarted(state, &room_id).await;
        }
    }
    Ok(())
}

async fn leave_room_if_needed(
    state: &AppState,
    current_room: &mut Option<SessionBinding>,
    user_id: u64,
) {
    if let Some(binding) = current_room.take() {
        let mut interrupted_player_count = None;
        {
            let mut rooms = state.rooms.lock().await;
            if let Some(result) = rooms.remove_connection(&binding.room_id, user_id) {
                if result.game_interrupted && !result.room_deleted {
                    interrupted_player_count = Some(result.player_count);
                }
            }
        }
        if let Some(player_count) = interrupted_player_count {
            broadcast_room_interrupted(state, &binding.room_id, user_id, player_count).await;
        }
        broadcast_room_state(state, &binding.room_id).await;
    }
}

async fn send_room_list(state: &AppState, tx: &mpsc::UnboundedSender<ServerMessage>) {
    let rooms = {
        let rooms = state.rooms.lock().await;
        rooms.room_summaries()
    };
    let _ = tx.send(ServerMessage::RoomsList { rooms });
}

async fn broadcast_room_state(state: &AppState, room_id: &str) {
    let rooms = state.rooms.lock().await;
    let connections = rooms.room_connections(room_id);
    if let Some(connections) = connections {
        for connection in connections {
            if let Some(snapshot) = rooms.snapshot_for(room_id, connection.id) {
                if let Some(tx) = connection.tx {
                    let _ = tx.send(ServerMessage::RoomState(snapshot));
                }
            }
        }
    }
}

async fn broadcast_game_over(state: &AppState, room_id: &str, winner_id: u64) {
    let rooms = state.rooms.lock().await;
    if let Some(connections) = rooms.room_connections(room_id) {
        for connection in connections {
            if let Some(tx) = connection.tx {
                let _ = tx.send(ServerMessage::GameOver {
                    room_id: room_id.to_string(),
                    winner_id,
                });
            }
        }
    }
}

async fn broadcast_room_interrupted(
    state: &AppState,
    room_id: &str,
    leaver_id: u64,
    player_count: usize,
) {
    let rooms = state.rooms.lock().await;
    if let Some(connections) = rooms.room_connections(room_id) {
        for connection in connections {
            if let Some(tx) = connection.tx {
                let _ = tx.send(ServerMessage::RoomInterrupted {
                    room_id: room_id.to_string(),
                    leaver_id,
                    player_count,
                });
            }
        }
    }
}

async fn broadcast_game_restarted(state: &AppState, room_id: &str) {
    let rooms = state.rooms.lock().await;
    if let Some(connections) = rooms.room_connections(room_id) {
        for connection in connections {
            if let Some(tx) = connection.tx {
                let _ = tx.send(ServerMessage::GameRestarted {
                    room_id: room_id.to_string(),
                });
            }
        }
    }
}
