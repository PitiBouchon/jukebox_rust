//! This is my version of Youtube-DL in Rust to search for videos and get the audio link of a video (see documentation)

#[cfg(test)]
mod test;
mod youtube_extractor;
pub mod youtube_info;

use crate::youtube_extractor::{ErrorExtractor, YtVideoPage};
use youtube_extractor::YtPageData;
use youtube_info::*;

/// To search for videos :
/// ```
/// use my_youtube_extractor::search_videos;
/// use my_youtube_extractor::youtube_info::YtVideoPageInfo;
///
/// // Async is just here to show it must be in an async block
/// async {
///     let v: Vec<YtVideoPageInfo> = search_videos("Diggy diggy hole").await?;
/// };
/// ```
/// See [`crate::youtube_info::YtVideoPageInfo`]
pub async fn search_videos(search: &str) -> Result<Vec<YtVideoPageInfo>, ErrorExtractor> {
    let url = "https://www.youtube.com/results?search_query=".to_owned() + search;

    YtPageData::new(url.as_str()).await?.videos_search_info()
}

/// To get the audio link of a video :
/// ```
/// use my_youtube_extractor::get_best_audio;
/// use my_youtube_extractor::youtube_info::YtAudioData;
///
/// // Async is just here to show it must be in an async block
/// async {
///     let link: YtAudioData = get_best_audio("ytWz0qVvBZ0").await.unwrap(); // Video link is : https://www.youtube.com/watch?v=ytWz0qVvBZ0
/// };
/// ```
/// See [`crate::youtube_info::YtAudioData`]
pub async fn get_best_audio(id: &str) -> Result<YtAudioData, ErrorExtractor> {
    let url = "https://www.youtube.com/watch?v=".to_owned() + id;
    let yt_video_page = YtVideoPage::new(url.as_str()).await?;
    yt_video_page.get_best_audio().await
}
