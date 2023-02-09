use crate::AppState;
use anyhow::Result;
use axum::extract::ws::{self, Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use entity::video::Model as Video;
use futures::stream::SplitSink;
use futures::{sink::SinkExt, stream::StreamExt};
use jukebox_rust::NetData;
use libmpv::FileState;
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
                Message::Binary(data) => match NetData::decode_message(data.as_slice()) {
                    Ok(msg) => match msg {
                        NetData::Remove(video_id) => {
                            log::debug!("Removing video: {}", video_id);
                            let mut playlist = state.list.lock().await;
                            if let Some((index, _)) =
                                playlist.iter().enumerate().find(|(_, m)| m.id == video_id)
                            {
                                playlist.remove(index);
                                let mpv_player = state.mpv.lock().await;
                                mpv_player.playlist_remove_index(index).unwrap();
                            }
                            state.tx.send(NetData::Remove(video_id)).unwrap();
                        }
                        NetData::Add(video) => {
                            log::debug!("Adding video: {}", video.title);
                            let mut playlist = state.list.lock().await;
                            playlist.push(video.clone());
                            let mpv_player = state.mpv.lock().await;
                            mpv_player
                                .playlist_load_files(&[(
                                    &format!("https://www.youtube.com/watch?v={}", video.id),
                                    if playlist.len() == 1 {
                                        FileState::AppendPlay
                                    } else {
                                        FileState::Append
                                    },
                                    Some("--vid=no"),
                                )])
                                .expect("Cannot play MPV Player");
                            state.tx.send(NetData::Add(video)).unwrap();
                        }
                        NetData::Search(search_txt) => {
                            log::debug!("Search videos: {search_txt}");
                            let videos = my_youtube_extractor::search_videos(&search_txt).await;
                            tx_single
                                .send(NetData::SearchResult(
                                    videos
                                        .iter()
                                        .map(|v| Video {
                                            id: v.id.to_owned(),
                                            title: v.title.to_owned(),
                                            author: v.author.name.to_owned(),
                                            thumbnail: v.thumbnail.to_owned(),
                                            duration: v.duration.clone(),
                                        })
                                        .collect(),
                                ))
                                .await
                                .unwrap();
                        }
                        NetData::Play => {
                            log::debug!("Play video");
                            let mpv_player = state.mpv.lock().await;
                            mpv_player.unpause().unwrap();
                        }
                        NetData::Pause => {
                            log::debug!("Pause video");
                            let mpv_player = state.mpv.lock().await;
                            mpv_player.pause().unwrap();
                        }
                        NetData::Next => {
                            log::debug!("Next video");
                            let mpv_player = state.mpv.lock().await;
                            mpv_player.playlist_next_force().unwrap();
                            tx_single.send(NetData::Next).await.unwrap();
                        }
                        NetData::SetVolume(volume) => {
                            let mpv_player = state.mpv.lock().await;
                            mpv_player.set_property("volume", volume).unwrap();
                        }
                        _ => ()
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

async fn send_data_ws(sender: &mut SplitSink<WebSocket, Message>, data: NetData) -> Result<()> {
    let msg = data.encode_message()?;
    sender.send(ws::Message::Binary(msg)).await?;
    Ok(())
}
