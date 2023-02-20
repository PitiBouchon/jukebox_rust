use crate::AppState;
use entity::video;
use futures::StreamExt;
use gstreamer::prelude::{ElementExt, ObjectExt};
use gstreamer::{glib, MessageView, State};
use jukebox_rust::NetData;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::log;

#[derive(Debug)]
pub enum MusicPlayerMessage {
    SetVolume(f64),
    AddMusic(video::Model),     // (video, APPEND_PLAY)
    RemoveVideo(usize, String), // Index and Id of the video in the playlist
    Play,
    Pause,
}

// Reference : https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/blob/7dc5a90b8ab45593d2461850d274ce8ca84891fe/examples/src/bin/glib-futures.rs

pub fn music_player(mut rx: UnboundedReceiver<MusicPlayerMessage>, app_state: Arc<AppState>) {
    let ctx = glib::MainContext::default();
    let main_loop = glib::MainLoop::new(Some(&ctx), false);
    gstreamer::init().expect("gstreamer initialization failed");

    // Used to play music
    let pipeline = gstreamer::parse_launch(&format!("playbin")).unwrap();
    // Used to receive events of the pipeline
    let bus = pipeline.bus().unwrap();

    // Spawn a new thread with tokio so that is have a tokio reactor (std::thread::spawn will not work here)
    tokio::task::spawn_blocking(move || {
        ctx.spawn_local(async move {
            let mut music_player_playlist: Vec<(String, String)> = vec![];
            let mut messages = bus.stream();
            loop {
                tokio::select! {
                    msg1_opt = rx.recv() => {
                        if let Some(msg) = msg1_opt {
                            match msg {
                                MusicPlayerMessage::SetVolume(volume) => {
                                    pipeline.set_property("volume", volume);
                                }
                                MusicPlayerMessage::AddMusic(video) => {
                                    if let Ok(video_data) = my_youtube_extractor::get_best_audio(&video.id).await {
                                        let uri = video_data.url;
                                        if music_player_playlist.is_empty() {
                                            log::info!("Playing music: {}", uri);
                                            pipeline.set_property("uri", uri.clone());
                                            pipeline.set_state(State::Playing).unwrap();
                                        }
                                        music_player_playlist.push((video.id, uri));
                                    }
                                }
                                MusicPlayerMessage::RemoveVideo(index, video_id) => {
                                    if let Some((local_video_id, _)) = music_player_playlist.first() && local_video_id == &video_id {
                                        music_player_playlist.remove(index);
                                        if index == 0 {
                                            if let Some((_, uri)) = music_player_playlist.first() {
                                                log::info!("Playing music: {}", uri);
                                                pipeline.set_property("uri", uri.clone());
                                                pipeline.set_state(State::Playing).unwrap();
                                            }
                                            else {
                                                pipeline.set_state(State::Null).unwrap();
                                            }
                                        }
                                    }
                                    else {
                                        log::error!("Trying to remove a video that is not in the playlist");
                                    }
                                }
                                MusicPlayerMessage::Play => {
                                    pipeline.set_state(State::Playing).unwrap();
                                }
                                MusicPlayerMessage::Pause => {
                                    pipeline.set_state(State::Paused).unwrap();
                                }
                            }
                        }
                    }
                    msg2_opt = messages.next() => {
                        if let Some(msg) = msg2_opt {
                            match msg.view() {
                                MessageView::Eos(..) => {
                                    let mut playlist_axum = app_state.list.lock().await;
                                    music_player_playlist.remove(0);
                                    playlist_axum.remove(0);
                                    app_state.tx.send(NetData::Next).unwrap();
                                    if let Some((_, uri)) = music_player_playlist.first() {
                                        log::info!("Playing music: {}", uri);
                                        pipeline.set_property("uri", uri.clone());
                                        pipeline.set_state(State::Playing).unwrap();
                                    }
                                    else {
                                        pipeline.set_state(State::Null).unwrap();
                                    }
                               }
                                _ => ()
                            }
                        }
                    }
                }
            }
        });

        main_loop.run();
    });
}
