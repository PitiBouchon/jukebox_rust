use crate::music_player::MusicPlayerMessage;
use crate::AppState;
use anyhow::Result;
use axum::extract::ws::{self, Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use entity::video::Model as Video;
use futures::stream::SplitSink;
use futures::{sink::SinkExt, stream::StreamExt};
use jukebox_rust::NetData;
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
            log::debug!("Received a message !");
            match msg {
                Message::Binary(data) => match NetData::decode_message(data.as_slice()) {
                    Ok(msg) => match msg {
                        NetData::Remove(index, video_id) => {
                            log::debug!("Removing video: {}", video_id);
                            let mut playlist = state.list.lock().await;
                            if let Some(video) = playlist.get(index) && video.id == video_id {
                                playlist.remove(index);
                                state.music_player_tx.send(MusicPlayerMessage::RemoveVideo(index, video_id.clone())).unwrap();
                                state.tx.send(NetData::Remove(index, video_id)).unwrap();
                            }
                            else {
                                log::error!("Trying to remove a video that is not in the playlist");
                            }
                        }
                        NetData::Add(video) => {
                            log::debug!("Adding video: {}", video.title);
                            let mut playlist = state.list.lock().await;
                            playlist.push(video.clone());
                            state
                                .music_player_tx
                                .send(MusicPlayerMessage::AddMusic(video.clone()))
                                .unwrap();
                            state.tx.send(NetData::Add(video)).unwrap();
                        }
                        NetData::Search(search_txt) => {
                            log::debug!("Search videos: {search_txt}");
                            match my_youtube_extractor::search_videos(&search_txt).await {
                                Err(why) => log::error!("Error searching videos : {}", why),
                                Ok(videos) => tx_single
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
                                    .unwrap(),
                            }
                        }
                        NetData::Move(index, video_id, delta) => {
                            let mut playlist = state.list.lock().await;
                            if index as i32 + delta >= 0 && index as i32 + delta < playlist.len() as i32 && let Some(video) = playlist.get(index) && video.id == video_id {
                                playlist.swap(index, (index as i32 + delta) as usize);
                                state
                                    .music_player_tx
                                    .send(MusicPlayerMessage::Move(index, video_id.clone(), delta))
                                    .unwrap();
                                state.tx.send(NetData::Move(index, video_id, delta)).unwrap();
                            }
                        }
                        NetData::Play => {
                            log::debug!("Play video");
                            state
                                .music_player_tx
                                .send(MusicPlayerMessage::Play)
                                .unwrap();
                        }
                        NetData::Pause => {
                            log::debug!("Pause video");
                            state
                                .music_player_tx
                                .send(MusicPlayerMessage::Pause)
                                .unwrap();
                            // let mpv_player = state.mpv.lock().await;
                            // mpv_player.pause().unwrap();
                        }
                        NetData::Next => {
                            log::debug!("Next video");
                            // TODO : Remove this
                            // let mpv_player = state.mpv.lock().await;
                            // mpv_player.playlist_next_force().unwrap();
                            // tx_single.send(NetData::Next).await.unwrap();
                        }
                        NetData::SetVolume(volume) => {
                            // let mpv_player = state.mpv.lock().await;
                            // mpv_player.set_property("volume", volume).unwrap();
                            state
                                .music_player_tx
                                .send(MusicPlayerMessage::SetVolume(volume))
                                .unwrap();
                        }
                        _ => (),
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
