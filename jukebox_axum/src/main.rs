mod login;
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
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::log;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct AppState {
    pub list: Mutex<Vec<YtVideoPageInfo>>,
    pub tx: broadcast::Sender<jukebox_rust::NetDataAxum>,
}

#[tokio::main]
async fn main() {
    // Tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jukebox_axum=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, _rx) = broadcast::channel(1000);

    // Channel between music player and axum web server
    let app_state = Arc::new(AppState {
        list: Mutex::new(vec![]),
        tx,
    });

    // Music player
    let music_player = async move {
        let mut interval = tokio::time::interval(Duration::from_secs_f32(5.0));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // tracing::debug!("20 seconds has passed");
                    // let mut playlist = app_state_copy.list.lock().await;
                    // if (playlist.len() > 0) {
                    //     playlist.remove(0);
                    // }
                }
            }
        }
    };

    tokio::spawn(music_player);

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

async fn playlist(State(app_state): State<Arc<AppState>>) -> Json<Vec<YtVideoPageInfo>> {
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
