use std::sync::Arc;
use libmpv::events::Event;
use libmpv::{FileState, Mpv};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::log;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::task::JoinHandle;
use entity::video;
use crate::AppState;

#[derive(Debug)]
pub enum MusicPlayerMessage {
    SetVolume(f64),
    AddMusic(video::Model), // (video, APPEND_PLAY)
    RemoveVideo(usize), // Index of the video in the playlist
    Play,
    Pause,
}

pub fn music_player(mut rx: UnboundedReceiver<MusicPlayerMessage>, app_state: Arc<AppState>) -> JoinHandle<()> {
    let mpv = Mpv::new().unwrap();
    let _ = mpv.set_property("vid", "no");
    let _ = mpv.set_property("af", "dynaudnorm"); // TODO : check if it change something

    tokio::spawn(async move {
        let mut ev_ctx = mpv.create_event_context();
        ev_ctx.disable_deprecated_events().unwrap();

        loop {
            let msg_res = rx.try_recv();
            match msg_res {
                Err(TryRecvError::Disconnected) => log::error!("Disconnected WTF ??"),
                Ok(msg) => {
                    match msg {
                        MusicPlayerMessage::Play => mpv.unpause().unwrap(),
                        MusicPlayerMessage::Pause => mpv.pause().unwrap(),
                        MusicPlayerMessage::SetVolume(volume) => mpv.set_property("volume", volume).unwrap(),
                        MusicPlayerMessage::AddMusic(video) => {
                            let append_play = app_state.list.lock().await.len() == 1;
                            mpv
                                .playlist_load_files(&[(
                                    &format!("https://www.youtube.com/watch?v={}", video.id),
                                    if append_play {
                                        FileState::AppendPlay
                                    } else {
                                        FileState::Append
                                    },
                                    Some("--vid=no"),
                                )])
                                .expect("Cannot play MPV Player");
                        }
                        MusicPlayerMessage::RemoveVideo(index) => {
                            if mpv.playlist_remove_index(index).is_ok() && !app_state.list.lock().await.is_empty() {
                                let _ = mpv.playlist_next_weak();
                            }
                        }
                    }
                },
                _ => ()
            }
            if let Some(Ok(ev)) = ev_ctx.wait_event(0.0) {
                log::debug!("Got event: {:?}", ev);
                match ev {
                    Event::Shutdown => {
                        log::error!("Shutdown player");
                    }
                    // Event::LogMessage { .. } => {}
                    // Event::GetPropertyReply { .. } => {}
                    // Event::SetPropertyReply(_) => {}
                    // Event::CommandReply(_) => {}
                    // Event::StartFile => {}
                    Event::EndFile(reason) => {
                        log::info!("End file reason: {:?}", reason);
                    }
                    // Event::FileLoaded => {}
                    // Event::ClientMessage(_) => {}
                    // Event::VideoReconfig => {}
                    // Event::AudioReconfig => {}
                    // Event::Seek => {}
                    // Event::PlaybackRestart => {}
                    // Event::PropertyChange { .. } => {}
                    // Event::QueueOverflow => {}
                    // Event::Deprecated(_) => {}
                    _ => ()
                }
            }
        }
    })
}
