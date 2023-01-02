#![allow(unused)] // While exploring, remove for prod.

// SurrealDB start code from https://github.com/jeremychone-channel/rust-surrealdb

use std::error::Error;
use std::sync::Arc;

// use anyhow::{anyhow, Result};
use axum::body::{BoxBody, Body, boxed};
use axum::http::{Uri, Response, StatusCode, Request};
use axum::{routing::get, Router, Extension};
use common::{FRUITS, GameState};
use surrealdb::{Datastore, Session};
use tower::ServiceExt;
use tower_http::services::ServeDir;


const PORT: u16 = 3000;

pub type DB = (Datastore, Session);

// #[derive(Clone)]
pub struct AppState {
    pub db: DB,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db: DB = (
        Datastore::new("file://temp.db").await?,
        Session::for_db("my_ns", "my_db"),
    );
    let (ds, ses) = &db;

    // // --- Create
    // let t1 = create_task(db, "Task 01", 10).await?;
    // let t2 = create_task(db, "Task 02", 7).await?;

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

    // build our application with a single route
    let state = Arc::new(AppState { db });
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .layer(Extension(GameState::new()))
        .with_state(state);

    // run it with hyper
    println!("Listening on port {PORT}");
    axum::Server::bind(&format!("0.0.0.0:{PORT}").parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// async fn handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
//     let res = get_static_file(uri.clone()).await?;

//     // if res.status() == StatusCode::NOT_FOUND {
//     //     // try with `.html`
//     //     // TODO: handle if the Uri has query parameters
//     //     match format!("{}.html", uri).parse() {
//     //         Ok(uri_html) => get_static_file(uri_html).await,
//     //         Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid URI".to_string())),
//     //     }
//     // } else {
//     //     Ok(res)
//     // }
//     Ok(res)
// }

// async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
//     let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

//     // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
//     match ServeDir::new(".").oneshot(req).await {
//         Ok(res) => Ok(res.map(boxed)),
//         Err(err) => Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             format!("Something went wrong: {}", err),
//         )),
//     }
// }

mod ws{
    use axum::{
        extract::ws::{WebSocketUpgrade, WebSocket, Message},
        response::{IntoResponse, Response}, Extension,
    };

    use crate::GameState;

    pub async fn ws_handler(ws: WebSocketUpgrade, Extension(game_state): Extension<GameState>) -> Response {
        ws.on_upgrade(|socket| handle_socket(socket, game_state))
    }

    async fn handle_socket(mut socket: WebSocket, game_state: GameState) {
        loop {
            if socket.send(Message::from(serde_json::to_string(&game_state).unwrap())).await.is_err() {
                // client disconnected
                return;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

mod api {
use std::collections::BTreeMap;
use anyhow::{anyhow, Result};
use surrealdb::sql::{thing, Datetime, Object, Thing, Value};
use surrealdb::{Datastore, Response, Session};

use crate::DB;

async fn create_task((ds, ses): &DB, title: &str, priority: i32) -> Result<String> {
    let sql = "CREATE task CONTENT $data";

    let data: BTreeMap<String, Value> = [
        ("title".into(), title.into()),
        ("priority".into(), priority.into()),
    ]
    .into();
    let vars: BTreeMap<String, Value> = [("data".into(), data.into())].into();

    let ress = ds.execute(sql, ses, Some(vars), false).await?;

    into_iter_objects(ress)?
        .next()
        .transpose()?
        .and_then(|obj| obj.get("id").map(|id| id.to_string()))
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