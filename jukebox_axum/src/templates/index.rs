use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub username: String,
    pub playlist: Vec<my_youtube_extractor::youtube_info::YtVideoPageInfo>,
    pub searched_musics: Vec<my_youtube_extractor::youtube_info::YtVideoPageInfo>,
}
