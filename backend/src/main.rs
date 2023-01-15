use std::error::Error;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::Redirect,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::routing::SpaRouter;
use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};
use common::{ChatMessage, DrawCanvas, GameState, LoginPost, Player, SetPixelPost};
use db::DB;
use rand::Rng;
use surrealdb::{Datastore, Session};
use tokio::sync::{
    broadcast::{channel, Sender},
    RwLock,
};

pub struct AppState {
    pub db: DB,
    pub game_state: RwLock<GameState>,
    pub game_channel: Sender<bool>,
    pub canvas_channel: Sender<bool>,
    pub chat_channel: Sender<ChatMessage>,
}

pub async fn build_app() -> Result<Router, Box<dyn Error>> {
    println!("Connecting to database");
    let database: DB = (
        Datastore::new("file://temp.db").await?,
        Session::for_db("my_ns", "my_db"),
    );

    // Cookie sessions
    let store = MemoryStore::new();
    let mut arr2 = [0u8; 128];
    rand::thread_rng().fill(&mut arr2);
    let session_layer = SessionLayer::new(store, &arr2);

    // Connections, state, and channels for the app
    let state = Arc::new(AppState {
        db: database,
        game_state: RwLock::new(GameState::new()),
        game_channel: channel(128).0,
        canvas_channel: channel(128).0,
        chat_channel: channel(128).0,
    });
    let app = Router::new()
        .route(
            "/ws/canvas",
            get(|w, s, e| ws::ws_handler(w, s, e, ws::WsStreamType::Canvas)),
        )
        .route(
            "/ws/game",
            get(|w, s, e| ws::ws_handler(w, s, e, ws::WsStreamType::Game)),
        )
        .route(
            "/ws/chat",
            get(|w, s, e| ws::ws_handler(w, s, e, ws::WsStreamType::Chat)),
        )
        .layer(Extension(state.clone()))
        .nest(
            "/api",
            Router::new()
                .route("/set_pixel", post(set_pixel_handler))
                .route("/clear_canvas", get(clear_canvas_handler))
                .route("/chat", post(chat_handler))
                .route("/player", get(get_player))
                .route("/register", post(register))
                .route("/login", post(login))
                .route(
                    "/logout",
                    get(|mut s: WritableSession| async move {
                        s.destroy();
                        Redirect::temporary("/")
                    }),
                )
                .nest(
                    "/gallery",
                    Router::new()
                        .route("/save", get(gallery_save_handler))
                        .route("/canvasses", get(gallery_canvasses_handler)),
                ),
        )
        .route("/favicon.ico", get(|| async move { StatusCode::NOT_FOUND }))
        .merge(SpaRouter::new("/assets", "frontend/dist").index_file("index.html"))
        .layer(session_layer)
        .with_state(state);
    Ok(app)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port: u16 = std::env::var("PORT")
        .unwrap_or("3000".into())
        .parse()
        .expect("Invalid PORT variable");

    let app = build_app().await?;

    let addr = format!("0.0.0.0:{port}").parse()?;
    println!("Listening on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn register(
    mut session: WritableSession,
    State(state): State<Arc<AppState>>,
    Json(LoginPost { username, password }): Json<LoginPost>,
) -> StatusCode {
    let x = db::create_user(&state.db, &username, &password)
        .await
        .unwrap();
    println!("User registered: {x}");
    session
        .insert(
            "user",
            Player {
                username,
                active: false,
            },
        )
        .expect("Insert fail");
    StatusCode::OK
}
async fn login(
    mut session: WritableSession,
    State(state): State<Arc<AppState>>,
    Json(LoginPost { username, password }): Json<LoginPost>,
) -> StatusCode {
    let username = db::auth_user(&state.db, &username, &password)
        .await
        .unwrap();
    session
        .insert(
            "user",
            Player {
                username,
                active: false,
            },
        )
        .expect("Insert fail");
    StatusCode::OK
}
async fn gallery_save_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    match verify_session(&session).await {
        StatusCode::UNAUTHORIZED => return StatusCode::UNAUTHORIZED,
        _ => (),
    };
    let player = session.get::<Player>("user").unwrap();
    let gs = { (*state.game_state.read().await).clone() };
    db::save_canvas(&state.db, &player.username, &gs.canvas)
        .await
        .unwrap();
    StatusCode::OK
}
async fn gallery_canvasses_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<DrawCanvas>> {
    match verify_session(&session).await {
        StatusCode::UNAUTHORIZED => return Json(vec![]),
        _ => (),
    };
    let player = session.get::<Player>("user").unwrap();
    let v = db::get_canvasses(&state.db, &player.username)
        .await
        .unwrap();
    Json(v)
}

async fn get_player(session: ReadableSession) -> Json<Option<Player>> {
    Json(session.get::<Player>("user"))
}

async fn verify_session(session: &ReadableSession) -> StatusCode {
    match session.get::<Player>("user") {
        Some(_) => StatusCode::OK,
        None => StatusCode::UNAUTHORIZED,
    }
}

async fn set_pixel_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
    Json(SetPixelPost { pixel_id, color }): Json<SetPixelPost>,
) -> StatusCode {
    match verify_session(&session).await {
        StatusCode::UNAUTHORIZED => return StatusCode::UNAUTHORIZED,
        _ => (),
    };
    {
        let mut gs = state.game_state.write().await;
        (*gs).canvas.set_pixel(pixel_id, color);
    }
    if state.canvas_channel.send(true).is_err() {
        println!("No receivers");
    }
    StatusCode::OK
}
async fn clear_canvas_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    match verify_session(&session).await {
        StatusCode::UNAUTHORIZED => return StatusCode::UNAUTHORIZED,
        _ => (),
    };
    {
        let mut gs = state.game_state.write().await;
        (*gs).canvas.clear();
    }
    if state.canvas_channel.send(true).is_err() {
        println!("No receivers");
    }
    StatusCode::OK
}
async fn chat_handler(
    session: ReadableSession,
    State(state): State<Arc<AppState>>,
    Json(chat_message): Json<ChatMessage>,
) -> StatusCode {
    match verify_session(&session).await {
        StatusCode::UNAUTHORIZED => return StatusCode::UNAUTHORIZED,
        _ => (),
    };
    let player = session.get::<Player>("user").unwrap();
    let username = player.username;
    let text = chat_message.text;
    let correct = {
        let gs = state.game_state.read().await;
        text.trim().to_lowercase() == (*gs).prompt
    };
    if state
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
        let mut gs = state.game_state.write().await;
        gs.new_round();
        if state.game_channel.send(true).is_err() {
            println!("No receivers");
        }
        if state.canvas_channel.send(true).is_err() {
            println!("No receivers");
        }
        if state
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
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        response::Response,
        Extension,
    };
    use axum_sessions::extractors::ReadableSession;
    use common::{ChatMessage, GameInfo, Player};

    use crate::AppState;

    pub enum WsStreamType {
        Game,
        Canvas,
        Chat,
    }

    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        session: ReadableSession,
        Extension(app_state): Extension<Arc<AppState>>,
        st: WsStreamType,
    ) -> Response {
        let player = session.get::<Player>("user");
        ws.on_upgrade(move |socket| handle_socket(socket, player, app_state, st))
    }

    async fn handle_socket(
        mut socket: WebSocket,
        player: Option<Player>,
        state: Arc<AppState>,
        st: WsStreamType,
    ) {
        let (new, player) = if let Some(player) = player {
            let mut gs = state.game_state.write().await;
            (gs.add_player(player.clone()), Some(player))
        } else {
            (false, None)
        };
        if new {
            if state.game_channel.send(true).is_err() {
                println!("No receivers");
            }
        }
        match st {
            WsStreamType::Canvas => {
                let mut rx = state.canvas_channel.subscribe();
                loop {
                    let gs = { (*state.game_state.read().await).clone() };
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
                let mut rx = state.game_channel.subscribe();
                loop {
                    let gs = { (*state.game_state.read().await).clone() };
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
                            serde_json::to_string(&GameInfo { prompt, players }).unwrap(),
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
                let mut rx = state.chat_channel.subscribe();
                if player.is_some() {
                    if state
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

mod db {
    // SurrealDB start code from https://github.com/jeremychone-channel/rust-surrealdb
    use anyhow::{anyhow, Result};
    use common::DrawCanvas;
    use std::collections::BTreeMap;
    use surrealdb::{
        sql::{thing, Object, Value},
        Datastore, Response, Session,
    };

    pub type DB = (Datastore, Session);

    pub async fn create_user((ds, ses): &DB, username: &str, password: &str) -> Result<String> {
        let sql =
            "CREATE user SET username = $username, password = crypto::scrypt::generate($password)";
        let vars: BTreeMap<String, Value> = [
            ("username".into(), username.into()),
            ("password".into(), password.into()),
        ]
        .into();
        let ress = ds.execute(sql, ses, Some(vars), false).await?;

        into_iter_objects(ress)?
            .next()
            .transpose()?
            .and_then(|obj| obj.get("id").map(|id| id.to_string()))
            .ok_or_else(|| anyhow!("No id returned."))
    }

    pub async fn auth_user((ds, ses): &DB, username: &str, password: &str) -> Result<String> {
        let sql = "SELECT * FROM user WHERE username = $username AND crypto::scrypt::compare(password, $password)";
        let vars: BTreeMap<String, Value> = [
            ("username".into(), username.into()),
            ("password".into(), password.into()),
        ]
        .into();
        let ress = ds.execute(sql, ses, Some(vars), false).await?;

        into_iter_objects(ress)?
            .next()
            .transpose()?
            .and_then(|obj| obj.get("username").map(|id| id.clone().as_string()))
            .ok_or_else(|| anyhow!("No id returned."))
    }

    pub async fn save_canvas((ds, ses): &DB, username: &str, canvas: &DrawCanvas) -> Result<()> {
        let sql = "CREATE canvas SET data = $data";
        let vars: BTreeMap<String, Value> =
            [("data".into(), serde_json::to_string(canvas)?.into())].into();
        let ress = ds.execute(sql, ses, Some(vars), false).await?;
        let id = into_iter_objects(ress)?
            .next()
            .transpose()?
            .and_then(|obj| obj.get("id").map(|id| id.clone().as_string()))
            .ok_or_else(|| anyhow!("No id returned."))?;
        let sql = "UPDATE user SET canvasses += $canvas_id WHERE username = $username";
        let vars: BTreeMap<String, Value> = [
            ("canvas_id".into(), thing(&id)?.into()),
            ("username".into(), username.into()),
        ]
        .into();
        let _ = ds.execute(sql, ses, Some(vars), false).await?;
        Ok(())
    }

    pub async fn get_canvasses((ds, ses): &DB, username: &str) -> Result<Vec<DrawCanvas>> {
        let sql = "SELECT canvasses FROM user WHERE username = $username FETCH canvasses";
        let vars: BTreeMap<String, Value> = [("username".into(), username.into())].into();
        let ress = ds.execute(sql, ses, Some(vars), false).await?;
        let x = into_iter_objects(ress)?
            .next()
            .transpose()?
            .and_then(|obj| {
                obj.get("canvasses").map(|c| match c {
                    Value::Array(vec) => vec
                        .iter()
                        .filter(|cv| match cv {
                            Value::Object(_) => true,
                            _ => false,
                        })
                        .map(|cv| match cv {
                            Value::Object(o) => serde_json::from_str::<DrawCanvas>(
                                &o.get("data").unwrap().clone().as_string(),
                            )
                            .map_err(|_| anyhow!("u suck")),
                            _ => Err(anyhow!("xdd")),
                        })
                        .collect(),
                    _ => vec![],
                })
            })
            .ok_or(anyhow!("xdd"))?
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        Ok(x)
    }

    fn into_iter_objects(ress: Vec<Response>) -> Result<impl Iterator<Item = Result<Object>>> {
        let res = ress.into_iter().next().map(|rp| rp.result).transpose()?;
        match res {
            Some(Value::Array(arr)) => {
                let it = arr.into_iter().map(|v| match v {
                    Value::Object(object) => Ok(object),
                    _ => Err(anyhow!("A record was not an Object")),
                });
                Ok(it)
            }
            _ => Err(anyhow!("No records found.")),
        }
    }
}
