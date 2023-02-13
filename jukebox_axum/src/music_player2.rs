use crate::music_player::MusicPlayerMessage;
use crate::AppState;
use gstreamer_player::{gst, PlayerVideoRenderer};
use std::sync::Arc;
use gstreamer::ClockTime;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tracing::log;
use jukebox_rust::NetData;

// #[derive(Debug)]
// pub enum MusicPlayerMessage {
//     SetVolume(f64),
//     AddMusic(video::Model), // (video, APPEND_PLAY)
//     RemoveVideo(usize), // Index of the video in the playlist
//     Play,
//     Pause,
// }

pub fn music_player(
    mut rx: UnboundedReceiver<MusicPlayerMessage>,
    app_state: Arc<AppState>,
) -> JoinHandle<()> {
    gst::init().expect("gstreamer initialization failed");
    let dispatcher = gstreamer_player::PlayerGMainContextSignalDispatcher::new(None);
    let video_renderer: Option<&PlayerVideoRenderer> = None;
    let player = gstreamer_player::Player::new(video_renderer, Some(&dispatcher));
    player.connect_end_of_stream(move |_player| log::error!("END OF STREAM"));
    player.connect_state_changed(|_player, _state| log::error!("State changed: {:?}", _state));

    tokio::spawn(async move {
        let mut player_playlist = vec![];
        loop {
            let msg_res = rx.try_recv();
            match msg_res {
                Err(TryRecvError::Disconnected) => log::error!("Disconnected WTF ??"),
                Ok(msg) => match msg {
                    MusicPlayerMessage::Play => player.play(),
                    MusicPlayerMessage::Pause => player.pause(),
                    MusicPlayerMessage::SetVolume(volume) => player.set_volume(volume),
                    MusicPlayerMessage::AddMusic(video) => {
                        let music_uri = my_youtube_extractor::get_best_audio(&video.id)
                            .await
                            .unwrap()
                            .url;
                        if player_playlist.is_empty() {
                            player.set_uri(Some(&music_uri));
                            player.play();
                        }
                        player_playlist.push(music_uri);
                        log::error!("Playing: {}", player_playlist.len());
                    }
                    MusicPlayerMessage::RemoveVideo(index) => {
                        player_playlist.remove(index);
                        if index == 0 {
                            if !player_playlist.is_empty() {
                                player.set_uri(Some(&player_playlist[0]));
                                player.play();
                            }
                            else {
                                player.set_uri(None);
                            }
                        }
                    }
                },
                _ => (),
            }
            const EPSILON: u64 = 10_000_000; // 10 microseconds epsilon
            if player.uri().is_some() && player.position().is_some_and(|player_position| {
                player.duration().is_some_and(|media_duration| {
                    player_position.abs_diff(*media_duration) < EPSILON && player_position != ClockTime::ZERO
                })
            }) {
                log::error!("Removing: {}", player_playlist.len());
                let mut playlist = app_state.list.lock().await;
                player_playlist.remove(0);
                playlist.remove(0);
                app_state.tx.send(NetData::Next).unwrap();
                if !player_playlist.is_empty() {
                    player.set_uri(Some(&player_playlist[0]));
                    player.play();
                }
                else {
                    player.seek(ClockTime::ZERO);
                    player.set_uri(None);
                }
            }
        }
    })
}
