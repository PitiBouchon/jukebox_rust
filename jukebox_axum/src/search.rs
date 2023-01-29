use std::sync::Arc;
use axum::extract::State;
use axum::Form;
use axum::response::{IntoResponse, Redirect};
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use serde::Deserialize;
use crate::{AppState, MusicChange, templates};
use crate::templates::index::IndexTemplate;

#[derive(Debug, Deserialize)]
pub struct SearchForm {
    pub search: String,
}

#[axum::debug_handler]
pub async fn search(
    State(app_state): State<Arc<AppState>>,
    search_form: Form<SearchForm>,
) -> impl IntoResponse {
    let videos = my_youtube_extractor::search_videos(&search_form.search).await;
    let playlist: Vec<YtVideoPageInfo> = app_state.list.lock().await.clone();
    let template = IndexTemplate {
        username: "User".to_string(),
        playlist,
        searched_musics: videos,
    };
    templates::HtmlTemplate(template)
}

#[derive(Debug, Deserialize)]
pub struct AddMusicForm {
    pub video_serialized: String,
}

#[axum::debug_handler]
pub async fn add_music(State(app_state): State<Arc<AppState>>, index_input: Form<AddMusicForm>) -> Redirect {
    // TODO : do better
    if let Ok(video) = serde_json::from_str::<YtVideoPageInfo>(&index_input.video_serialized[1..]) { // Remove the first '#' caracter
        let mut added_video = false;
        let mut playlist = app_state.list.lock().await;
        added_video = true;
        tracing::debug!("Video Add to playlist");
        playlist.push(video.clone());
    }
    else {
        tracing::warn!("Canno't parse the video : {}", index_input.video_serialized);
    }
    Redirect::to("/")
}
