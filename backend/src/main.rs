#![allow(unused)] // While exploring, remove for prod.

// SurrealDB start code from https://github.com/jeremychone-channel/rust-surrealdb

use std::error::Error;
use std::sync::Arc;

use axum::{
    body::{boxed, Body, BoxBody},
    extract::{Path, State},
    http::{Request, Response, StatusCode, Uri},
    response::Redirect,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};
use common::{ChatMessage, GameState, LoginPost, Player, SetPixelPost, FRUITS};
use surrealdb::{Datastore, Session};
use tokio::sync::broadcast::channel;
use tokio::sync::{broadcast::Sender, RwLock};
use tower::ServiceExt;
use tower_http::services::ServeDir;

const PORT: u16 = 3000;

pub type DB = (Datastore, Session);

pub struct AppState {
    pub db: DB,
    pub game_state: RwLock<GameState>,
    pub game_channel: Sender<bool>,
    pub canvas_channel: Sender<bool>,
    pub chat_channel: Sender<ChatMessage>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db: DB = (
        Datastore::new("file://temp.db").await?,
        Session::for_db("my_ns", "my_db"),
    );
    let (ds, ses) = &db;

    // --- Create
    // let t1 = database::create_task(&db, "Task 01", 10).await?;
    // let t2 = database::create_task(&db, "Task 02", 7).await?;

    // // --- Merge
    // let sql = "UPDATE $th MERGE $data RETURN id";
    // let data: BTreeMap<String, Value> = [
    //     ("title".into(), "Task 02 UPDATED".into()),
    //     ("done".into(), true.into()),
    // ]
    // .into();
    // let vars: BTreeMap<String, Value> = [
    //     ("th".into(), thing(&t2)?.into()),
    //     ("data".into(), data.into()),
    // ]
    // .into();
    // ds.execute(sql, ses, Some(vars), true).await?;

    // // --- Delete
    // let sql = "DELETE $th";
    // let vars: BTreeMap<String, Value> = [("th".into(), thing(&t1)?.into())].into();
    // ds.execute(sql, ses, Some(vars), true).await?;

    // // --- Select
    // let sql = "SELECT * from task";
    // let ress = ds.execute(sql, ses, None, false).await?;
    // for object in into_iter_objects(ress)? {
    //     println!("record {}", object?);
    // }

    let store = MemoryStore::new();
    let secret = include_bytes!("../../secret"); // MUST be at least 64 bytes!
    let session_layer = SessionLayer::new(store, secret);

    let state = Arc::new(AppState {
        db,
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
                ),
        )
        .route("/*file", get(get_static_file))
        .route("/", get(get_static_file))
        .layer(session_layer)
        .with_state(state);

    println!("Listening on port {PORT}");
    axum::Server::bind(&format!("0.0.0.0:{PORT}").parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn register(
    mut session: WritableSession,
    State(state): State<Arc<AppState>>,
    Json(LoginPost { username, password }): Json<LoginPost>,
) -> StatusCode {
    let x = database::create_user(&state.db, &username, &password)
        .await
        .unwrap();
    println!("{x}");
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
    let username = database::auth_user(&state.db, &username, &password)
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
        text.trim() == &(*gs).prompt
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
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            State,
        },
        response::{IntoResponse, Response},
        Extension,
    };
    use axum_sessions::extractors::ReadableSession;
    use common::{ChatMessage, GameInfo, Player};
    use tokio::sync::RwLock;

    use crate::{AppState, GameState};

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
                    let mut gs = { (*state.game_state.read().await).clone() };
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
            _ => (),
        };
    }
}

async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    match ServeDir::new("./frontend/dist").oneshot(req).await {
        Ok(res) => Ok(res.map(boxed)),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", err),
        )),
    }
}

mod database {
    use anyhow::{anyhow, Result};
    use std::collections::BTreeMap;
    use surrealdb::{
        sql::{thing, Datetime, Object, Thing, Value},
        Datastore, Response, Session,
    };

    use crate::DB;

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

    /// Returns Result<impl Iterator<Item = Result<Object>>>
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
