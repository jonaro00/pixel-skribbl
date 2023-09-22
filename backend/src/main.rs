use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};
use rand::Rng;
use shuttle_axum::ShuttleAxum;
use tokio::sync::{
    broadcast::{channel, Sender},
    RwLock,
};
use tower_http::services::{ServeDir, ServeFile};

use common::{ChatMessage, GameState, JoinLobbyPost, Player, SessionPlayer, SetPixelPost};

#[shuttle_runtime::main]
async fn axum() -> ShuttleAxum {
    let app = build_app().await?;
    Ok(app.into())
}

#[derive(Default)]
pub struct AppState {
    pub rooms: RwLock<HashMap<u32, Arc<RoomState>>>,
}

pub struct RoomState {
    pub room_id: String,
    pub game_state: RwLock<GameState>,
    pub game_channel: Sender<bool>,
    pub canvas_channel: Sender<bool>,
    pub chat_channel: Sender<ChatMessage>,
}
impl RoomState {
    fn new(room_id: String) -> Self {
        Self {
            room_id,
            game_state: RwLock::new(GameState::new()),
            game_channel: channel(128).0,
            canvas_channel: channel(128).0,
            chat_channel: channel(128).0,
        }
    }
}

pub async fn build_app() -> Result<Router> {
    // Cookie sessions
    let store = MemoryStore::new();
    let mut arr2 = [0u8; 128];
    rand::thread_rng().fill(&mut arr2);
    let session_layer = SessionLayer::new(store, &arr2);

    // Connections, state, and channels for the app
    let state: Arc<AppState> = Arc::new(Default::default());
    let app = Router::new()
        .route(
            "/ws/:room_id/canvas",
            get(|w, s, p, e| ws::ws_handler(w, s, p, e, ws::WsStreamType::Canvas)),
        )
        .route(
            "/ws/:room_id/game",
            get(|w, s, p, e| ws::ws_handler(w, s, p, e, ws::WsStreamType::Game)),
        )
        .route(
            "/ws/:room_id/chat",
            get(|w, s, p, e| ws::ws_handler(w, s, p, e, ws::WsStreamType::Chat)),
        )
        .layer(Extension(state.clone()))
        .nest(
            "/api",
            Router::new()
                .route("/create_lobby", post(create_lobby))
                .route("/join_lobby/:room_id", post(join_lobby))
                .route("/leave_lobby", get(leave_lobby))
                .route("/player", get(get_player_name))
                .route("/set_pixel", post(set_pixel_handler))
                .route("/clear_canvas", get(clear_canvas_handler))
                .route("/chat", post(chat_handler)),
        )
        .route("/favicon.ico", get(|| async move { StatusCode::NOT_FOUND }))
        .nest_service(
            "/",
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html")),
        )
        .layer(session_layer)
        .with_state(state);
    Ok(app)
}

async fn create_lobby(
    mut session: WritableSession,
    State(state): State<Arc<AppState>>,
    Json(JoinLobbyPost { username }): Json<JoinLobbyPost>,
) -> String {
    let code: u32 = rand::random();
    {
        let mut rooms = state.rooms.write().await;
        rooms.insert(code, Arc::new(RoomState::new(format!("{code}"))));
    }
    session
        .insert(
            "user",
            SessionPlayer {
                username,
                room: code,
            },
        )
        .unwrap();
    format!("{code}")
}

async fn join_lobby(
    mut session: WritableSession,
    Path(room_id): Path<u32>,
    Json(JoinLobbyPost { username }): Json<JoinLobbyPost>,
) -> StatusCode {
    session
        .insert(
            "user",
            SessionPlayer {
                username,
                room: room_id,
            },
        )
        .unwrap();
    StatusCode::OK
}

async fn leave_lobby(mut session: WritableSession, State(state): State<Arc<AppState>>) -> Redirect {
    let player = session.get::<SessionPlayer>("user");
    session.destroy();
    if let Some(player) = player {
        {
            let mut rooms = state.rooms.write().await;
            let room = match rooms.get(&player.room) {
                Some(r) => r.clone(),
                None => return Redirect::to("/"),
            };
            let mut gs = room.game_state.write().await;
            let advance = gs.remove_player(Player {
                username: player.username,
                active: false,
            });
            if gs.players.is_empty() {
                rooms.remove(&player.room);
                return Redirect::to("/");
            }
            if advance {
                gs.new_round();
            }
            if room.game_channel.send(true).is_err() {
                println!("No receivers");
            }
        }
    }
    Redirect::to("/")
}

async fn get_player_name(session: ReadableSession) -> Json<Option<String>> {
    Json(session.get::<SessionPlayer>("user").map(|p| p.username))
}

async fn verify_session(session: &ReadableSession) -> Result<SessionPlayer> {
    session
        .get::<SessionPlayer>("user")
        .ok_or(anyhow!("no player in this session"))
}

async fn set_pixel_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
    Json(SetPixelPost { pixel_id, color }): Json<SetPixelPost>,
) -> StatusCode {
    let player = match verify_session(&session).await {
        Ok(p) => p,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    let rooms = state.rooms.read().await;
    let room = match rooms.get(&player.room) {
        Some(r) => r.clone(),
        None => return StatusCode::NOT_FOUND,
    };
    {
        let mut gs = room.game_state.write().await;
        gs.canvas.set_pixel(pixel_id, color);
    }
    if room.canvas_channel.send(true).is_err() {
        println!("No receivers");
    }
    StatusCode::OK
}
async fn clear_canvas_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    let player = match verify_session(&session).await {
        Ok(p) => p,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    let rooms = state.rooms.read().await;
    let room = match rooms.get(&player.room) {
        Some(r) => r.clone(),
        None => return StatusCode::NOT_FOUND,
    };
    {
        let mut gs = room.game_state.write().await;
        gs.canvas.clear();
    }
    if room.canvas_channel.send(true).is_err() {
        println!("No receivers");
    }
    StatusCode::OK
}
async fn chat_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
    Json(chat_message): Json<ChatMessage>,
) -> StatusCode {
    let player = match verify_session(&session).await {
        Ok(p) => p,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    let rooms = state.rooms.read().await;
    let room = match rooms.get(&player.room) {
        Some(r) => r.clone(),
        None => return StatusCode::NOT_FOUND,
    };
    let username = player.username;
    let text = chat_message.text;
    let correct = {
        let gs = room.game_state.read().await;
        text.trim().to_lowercase() == gs.prompt
    };
    if room
        .chat_channel
        .send(ChatMessage {
            username: username.clone(),
            text,
        })
        .is_err()
    {
        println!("No receivers");
    }
    if correct {
        let mut gs = room.game_state.write().await;
        gs.new_round();
        if room.game_channel.send(true).is_err() {
            println!("No receivers");
        }
        if room.canvas_channel.send(true).is_err() {
            println!("No receivers");
        }
        if room
            .chat_channel
            .send(ChatMessage {
                username: "SYSTEM".into(),
                text: format!("{username} guessed the right word!"),
            })
            .is_err()
        {
            println!("No receivers");
        }
    }
    StatusCode::OK
}

mod ws {
    use std::sync::Arc;

    use axum::{
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            Path,
        },
        response::Response,
        Extension,
    };
    use axum_sessions::extractors::ReadableSession;
    use common::{ChatMessage, GameInfo, Player, SessionPlayer};

    use crate::AppState;

    pub enum WsStreamType {
        Game,
        Canvas,
        Chat,
    }

    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        session: ReadableSession,
        Path(room_id): Path<u32>,
        Extension(app_state): Extension<Arc<AppState>>,
        st: WsStreamType,
    ) -> Response {
        let player = session.get::<SessionPlayer>("user");
        ws.on_upgrade(move |socket| handle_socket(socket, player, room_id, app_state, st))
    }

    async fn handle_socket(
        mut socket: WebSocket,
        player: Option<SessionPlayer>,
        room_id: u32,
        state: Arc<AppState>,
        st: WsStreamType,
    ) {
        let rooms = state.rooms.read().await;
        let room = match rooms.get(&room_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let (new, player) = if let Some(player) = player {
            let mut gs = room.game_state.write().await;
            let player = Player {
                username: player.username,
                active: false,
            };
            (gs.add_player(player.clone()), Some(player))
        } else {
            (false, None)
        };
        if new {
            if room.game_channel.send(true).is_err() {
                println!("No receivers");
            }
        }
        match st {
            WsStreamType::Canvas => {
                let mut rx = room.canvas_channel.subscribe();
                loop {
                    let gs = { (*room.game_state.read().await).clone() };
                    if socket
                        .send(Message::from(serde_json::to_string(&gs.canvas).unwrap()))
                        .await
                        .is_err()
                    {
                        // client disconnected
                        return;
                    }
                    rx.recv().await.expect("Channel recv error");
                }
            }
            WsStreamType::Game => {
                let mut rx = room.game_channel.subscribe();
                loop {
                    let gs = { (*room.game_state.read().await).clone() };
                    let prompt = if !player
                        .clone()
                        .map(|ps| gs.players.iter().find(|p| **p == ps).unwrap().active)
                        .unwrap_or(false)
                    {
                        gs.prompt.replace(|c: char| c.is_alphabetic(), "_")
                    } else {
                        gs.prompt
                    };
                    let players = gs.players;
                    if socket
                        .send(Message::from(
                            serde_json::to_string(&GameInfo {
                                room_id: room.room_id.clone(),
                                prompt,
                                players,
                            })
                            .unwrap(),
                        ))
                        .await
                        .is_err()
                    {
                        // client disconnected
                        return;
                    }
                    rx.recv().await.expect("Channel recv error");
                }
            }
            WsStreamType::Chat => {
                let mut rx = room.chat_channel.subscribe();
                if player.is_some() {
                    if room
                        .chat_channel
                        .send(ChatMessage {
                            username: "SYSTEM".into(),
                            text: format!("{} joined!", player.clone().unwrap().username),
                        })
                        .is_err()
                    {
                        println!("No receivers");
                    }
                }
                loop {
                    let msg = rx.recv().await.expect("Channel recv error");
                    {
                        if socket
                            .send(Message::from(serde_json::to_string(&msg).unwrap()))
                            .await
                            .is_err()
                        {
                            // client disconnected
                            return;
                        }
                    }
                }
            }
        };
    }
}
