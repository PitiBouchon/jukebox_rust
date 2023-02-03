use crate::AppState;
use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::stream::SplitSink;
use futures::{sink::SinkExt, stream::StreamExt};
use jukebox_rust::{NetDataAxum, NetDataYew};
use my_youtube_extractor::search_videos;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::log;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = state.tx.subscribe();
    let (tx_single, mut rx_single) = mpsc::channel(1000);

    let mut recv_user_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(data) => match NetDataYew::decode_message(data.as_slice()) {
                    Ok(msg) => match msg {
                        NetDataYew::Remove(video_id) => {
                            log::debug!("Removing video: {}", video_id);
                            let mut playlist = state.list.lock().await;
                            if let Some((index, _)) =
                                playlist.iter().enumerate().find(|(_, m)| m.id == video_id)
                            {
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
                            tx_single.send(NetDataAxum::Search(videos)).await.unwrap();
                        }
                    },
                    Err(err) => log::error!("Error decoding message: {err}"),
                },
                Message::Close(_) => return,
                _ => log::error!("Received unwanted data: {msg:?}"),
            }
        }
    });

    let mut broadcast_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                data_res = rx.recv() => {
                    if let Ok(data) = data_res {
                        if let Err(err) = send_data_ws(&mut sender, data).await {
                            log::error!("Error sending data: {err}");
                            break;
                        }
                    }
                },
                data_opt = rx_single.recv() => {
                    if let Some(data) = data_opt {
                        if let Err(err) = send_data_ws(&mut sender, data).await {
                            log::error!("Error sending data: {err}");
                            break;
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut broadcast_task) => recv_user_task.abort(),
        _ = (&mut recv_user_task) => broadcast_task.abort(),
    }
}

async fn send_data_ws(sender: &mut SplitSink<WebSocket, Message>, data: NetDataAxum) -> Result<()> {
    let msg = data.encode_axum_message()?;
    sender.send(msg).await?;
    Ok(())
}
