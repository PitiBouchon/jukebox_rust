use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Formatter;
use bincode::{Decode, Encode};
use serde::ser::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct YtAuthorInfo {
    pub name: String,
    pub thumbnail: String,
    pub tag: String, // Verified, Music...
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct YtVideoPageInfo {
    pub id: String,
    pub short_recap: String,
    pub title: String,
    pub thumbnail: String,
    pub author: YtAuthorInfo,
    pub meta_description: String,
    pub duration: String,
    pub n_views: String,
    pub date: String, // TODO("Maybe change String to Date")
}

impl PartialEq for YtVideoPageInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for YtVideoPageInfo {}

impl std::fmt::Display for YtVideoPageInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(str_fmt) => write!(f, "{str_fmt}"),
            Err(e) => Err(std::fmt::Error::custom(e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct YtAudioData {
    pub url: String,
    pub itag: u32,
    pub sample_rate: u32,
    pub bitrate: u32,
    pub channels: u16,
    pub ms_duration: Option<u64>,
    pub loudness_db: Option<f32>,
}

impl std::fmt::Display for YtAudioData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let duration = match self.ms_duration {
            None => String::new(),
            Some(d) => format!("ms_duration: {d}"),
        };
        let loudness = match self.loudness_db {
            None => String::new(),
            Some(l) => format!("loudness_db: {l}"),
        };
        write!(
            f,
            "{{ url: {}, \
            itag: {}, \
            sample_rate: {}, \
            bitrate: {} \
            channels: {} \
            {}, \
            {} }}",
            self.url, self.itag, self.sample_rate, self.bitrate, self.channels, duration, loudness
        )
    }
}

impl YtAudioData {
    pub fn new(url: String, v: &Value) -> Result<Self, String> {
        if v.get("itag").is_none()
            || v.get("bitrate").is_none()
            || v.get("audioSampleRate").is_none()
            || v.get("audioChannels").is_none()
        {
            return Err(format!("{v} is not a normal video"));
        }

        let loudness_db = v.get("loudnessDb").map(|a| a.as_f64().unwrap() as f32);

        let ms_duration = v
            .get("approxDurationMs")
            .map(|a| a.as_str().unwrap().parse().unwrap());

        Ok(Self {
            url,
            itag: v.get("itag").unwrap().as_u64().unwrap() as u32,
            sample_rate: v.get("bitrate").unwrap().as_u64().unwrap() as u32,
            bitrate: v
                .get("audioSampleRate")
                .unwrap()
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            channels: v.get("audioChannels").unwrap().as_u64().unwrap() as u16,
            ms_duration,
            loudness_db,
        })
    }
}
