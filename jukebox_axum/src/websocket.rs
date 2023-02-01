use std::sync::Arc;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use futures::{sink::SinkExt, stream::StreamExt};
use futures::stream::iter;
use tracing::log;
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use crate::AppState;

pub enum NetData {
    Remove(YtVideoPageInfo),
    Add(YtVideoPageInfo),
    Search(Vec<YtVideoPageInfo>),
}

fn video_data_to_string(video_info: &YtVideoPageInfo) -> String {
    format!("({};{};{})", video_info.id, video_info.title, video_info.thumbnail)
}

impl ToString for NetData {
    fn to_string(&self) -> String {
        match self {
            NetData::Remove(v) => format!("rem {}", video_data_to_string(v)),
            NetData::Add(v) => format!("add {}", video_data_to_string(v)),
            NetData::Search(s) => {
                let videos_stringified: Vec<String> = s.iter().map(video_data_to_string).collect();
                let res = videos_stringified.join("|");
                format!("sch [{res}]")
            }
        }
    }
}

impl NetData {
    pub fn to_message(&self) -> Message {
        Message::Text(self.to_string())
    }
}

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
        while let Some(Ok(data)) = receiver.next().await {
            if let Message::Text(data) = data {
                log::debug!("Received something: {}", data);
                if data.len() >= 3 {
                    match &data[..3] {
                        "tes" => log::info!("received a tes"),
                        _ => {
                            tracing::info!("REMOVE SOMETHING: {:?}", data);
                            let video_id = &data[7..];
                            let mut playlist = state.list.lock().await;
                            if let Some((index, _)) = playlist.iter().enumerate().find(|(_, m)| m.id == video_id) {
                                playlist.remove(index);
                                state.tx.send(format!("rem {video_id}")).unwrap();
                            }
                        },
                    }
                }
            }
        }
    });

    let mut broadcast_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut broadcast_task) => recv_user_task.abort(),
        _ = (&mut recv_user_task) => broadcast_task.abort(),
    }
}
