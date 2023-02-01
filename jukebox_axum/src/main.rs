#![feature(let_chains)]
#[allow(unused_imports)] // remove useless warning for developping

#[allow(unused_imports)]
mod search;
#[allow(unused_imports)]
mod templates;
#[allow(unused_imports)]
mod websocket;

use std::convert::Infallible;
use crate::templates::index::IndexTemplate;
use axum::body::{boxed, Body};
use axum::extract::State;
use axum::http::Response;
use axum::http::StatusCode;
use axum::response::{Redirect, Sse};
use axum::{response::IntoResponse, routing::{get, post}, Router, Server, Json};
use my_youtube_extractor::youtube_info::{YtAuthorInfo, YtVideoPageInfo};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use axum::response::sse::Event;
use futures::{Stream, stream};
use tokio::sync::{broadcast, Mutex, mpsc};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::services::ServeDir;
use tracing::log;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug)]
pub struct AppState {
    pub list: Mutex<Vec<YtVideoPageInfo>>,
    pub tx: broadcast::Sender<jukebox_rust::NetDataAxum>,
}

#[derive(Debug)]
pub enum MusicChange {
    NewMusic(YtVideoPageInfo),
    RemoveMusic(YtVideoPageInfo),
}

// #[derive(Debug)]
// pub enum MusicPlayerChange {
//     PlayFirstMusic,
//     FinishPlaying,
// }

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
    // let (mut send_new_music, mut receive_new_music): (Sender<MusicChange>, Receiver<MusicChange>) = tokio::sync::mpsc::channel(10);
    // let (mut send_player_update, mut receive_player_update): (Sender<MusicPlayerChange>, Receiver<MusicPlayerChange>) = tokio::sync::mpsc::channel(10);

    let app_state_copy = app_state.clone();

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
        // .route("/", get(|| async { Redirect::permanent("/index") }))
        // .route("/index", get(main_page))
        .fallback_service(get(|req| async move {
            match ServeDir::new("jukebox_yew/dist/").oneshot(req).await {
                Ok(res) => {
                    let status = res.status();
                    match status {
                        StatusCode::NOT_FOUND => {
                            let index_path =
                                PathBuf::from("jukebox_yew/dist/").join("index.html");
                            let index_content = match tokio::fs::read_to_string(index_path).await {
                                Ok(index_content) => index_content,
                                Err(_) => {
                                    return Response::builder()
                                        .status(StatusCode::NOT_FOUND)
                                        .body(boxed(Body::from("index file not found")))
                                        .unwrap()
                                }
                            };

                            Response::builder()
                                .status(StatusCode::OK)
                                .body(boxed(Body::from(index_content)))
                                .unwrap()
                        }
                        _ => res.map(boxed),
                    }
                }
                Err(err) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(boxed(Body::from(format!("error: {err}"))))
                    .expect("error response"),
            }
        }))
        // .route("/search", post(search::search))
        // .route("/add_music", post(search::add_music))
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

async fn main_page(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let playlist = app_state.list.lock().await;
    let template = IndexTemplate {
        username: "User".to_string(),
        playlist: playlist.clone(),
        searched_musics: vec![],
    };
    templates::HtmlTemplate(template)
}

async fn playlist(State(app_state): State<Arc<AppState>>) -> Json<Vec<YtVideoPageInfo>> {
    log::info!("Get /api/playlist");
    let playlist = app_state.list.lock().await;
    Json(playlist.clone())
}
