use std::sync::Arc;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::log;
use jukebox_rust::{NetDataAxum, NetDataYew};
use my_youtube_extractor::search_videos;
use crate::AppState;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = state.tx.subscribe();

    let mut recv_user_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(data) => {
                    match NetDataYew::decode_message(data.as_slice()) {
                        Ok(msg) => {
                            match msg {
                                NetDataYew::Remove(video_id) => {
                                    log::debug!("Removing video: {}", video_id);
                                    let mut playlist = state.list.lock().await;
                                    if let Some((index, _)) = playlist.iter().enumerate().find(|(_, m)| m.id == video_id) {
                                        playlist.remove(index);
                                    }
                                    state.tx.send(NetDataAxum::Remove(video_id)).unwrap();
                                }
                                NetDataYew::Add(video) => {
                                    log::debug!("Adding video: {}", video.title);
                                    let mut playlist = state.list.lock().await;
                                    playlist.push(video.clone());
                                    state.tx.send(NetDataAxum::Add(video)).unwrap();
                                }
                                NetDataYew::Search(search_txt) => {
                                    log::debug!("Search videos: {search_txt}");
                                    let videos = search_videos(&search_txt).await;
                                    state.tx.send(NetDataAxum::Search(videos)).unwrap();
                                }
                            }
                        }
                        Err(err) => log::error!("Error decoding message: {err}"),
                    }
                }
                _ => log::error!("Received unwanted data: {msg:?}"),
            }
            // if let Message::Text(data) = data {
            //     log::debug!("Received something: {}", data);
            //     if data.len() >= 3 {
            //         match &data[..3] {
            //             "tes" => log::info!("received a tes"),
            //             _ => {
            //                 tracing::info!("REMOVE SOMETHING: {:?}", data);
            //                 let video_id = &data[7..];
            //                 let mut playlist = state.list.lock().await;
            //                 if let Some((index, _)) = playlist.iter().enumerate().find(|(_, m)| m.id == video_id) {
            //                     playlist.remove(index);
            //                     state.tx.send(format!("rem {video_id}")).unwrap();
            //                 }
            //             },
            //         }
            //     }
            // }
        }
    });

    let mut broadcast_task = tokio::spawn(async move {
        while let Ok(data) = rx.recv().await {
            match data.encode_axum_message() {
                Ok(msg) => {
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(err) => log::error!("Error encoding data: {err}"),
            }
        }
    });

    tokio::select! {
        _ = (&mut broadcast_task) => recv_user_task.abort(),
        _ = (&mut recv_user_task) => broadcast_task.abort(),
    }
}
