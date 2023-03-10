#![feature(is_some_and)]
#![feature(let_chains)]

mod login;
mod music_player;
mod sql;
mod templates;
mod websocket;

use crate::login::jwt_token::AuthToken;
use crate::login::{authorize, login_page, register_page, register_post};
use axum::body::{boxed, Body};
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::{Request, Response};
use axum::response::{IntoResponse, Redirect};
use axum::{routing::get, Json, Router, Server};
use entity::user;
use entity::video;
use music_player::MusicPlayerMessage;
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, DbConn, Schema};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{broadcast, Mutex};
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::log;

pub struct AppState {
    pub list: Mutex<Vec<video::Model>>,
    pub tx: broadcast::Sender<jukebox_rust::NetData>,
    pub conn: DatabaseConnection,
    pub music_player_tx: UnboundedSender<MusicPlayerMessage>,
}

async fn setup_schema(db: &DbConn) {
    // Setup Schema helper
    let schema = Schema::new(DbBackend::Sqlite);

    let stmt: TableCreateStatement = schema.create_table_from_entity(user::Entity);
    let _ = db.execute(db.get_database_backend().build(&stmt)).await;

    let stmt: TableCreateStatement = schema.create_table_from_entity(video::Entity);
    let _ = db.execute(db.get_database_backend().build(&stmt)).await;
}

#[tokio::main]
async fn main() {
    // Tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();

    let (tx, _rx) = broadcast::channel(1000);

    // Channel between music player and axum web server
    let conn = Database::connect(format!("sqlite://{}?mode=rwc", "sqlite.db"))
        .await
        .expect("Database connection failed");
    setup_schema(&conn).await;

    let (music_player_tx, rx1) = tokio::sync::mpsc::unbounded_channel();
    let app_state = Arc::new(AppState {
        list: Mutex::new(vec![]),
        tx,
        conn,
        music_player_tx,
    });

    music_player::music_player(rx1, app_state.clone());

    // Axum web server
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/index") }))
        .route("/login", get(login_page).post(authorize))
        .route("/register", get(register_page).post(register_post))
        .fallback_service(tower::service_fn(fallback_service_fn))
        .route("/websocket", get(websocket::websocket_handler))
        .route("/api/playlist", get(playlist))
        .with_state(app_state);

    let addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();
    tracing::info!("Starting server on http://{addr}/index");

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn playlist(State(app_state): State<Arc<AppState>>) -> Json<Vec<video::Model>> {
    log::info!("Get /api/playlist");
    let playlist = app_state.list.lock().await;
    Json(playlist.clone())
}

async fn fallback_service_fn(request: Request<Body>) -> Result<impl IntoResponse, Infallible> {
    match AuthToken::from_request(&request).await {
        Ok(_token) => match ServeDir::new("jukebox_yew/dist/").oneshot(request).await {
            Ok(res) => {
                let status = res.status();
                match status {
                    StatusCode::NOT_FOUND => {
                        let index_path = PathBuf::from("jukebox_yew/dist/").join("index.html");
                        let index_content = match tokio::fs::read_to_string(index_path).await {
                            Ok(index_content) => index_content,
                            Err(_) => {
                                return Ok(Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .body(boxed(Body::from("index file not found")))
                                    .unwrap())
                            }
                        };

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .body(boxed(Body::from(index_content)))
                            .unwrap())
                    }
                    _ => Ok(res.map(boxed)),
                }
            }
            Err(err) => Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(boxed(Body::from(format!("error: {err}"))))
                .expect("error response")),
        },
        Err(_) => Ok(Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header("location", "/login")
            .body(boxed(Body::empty()))
            .unwrap()),
    }
}
