use anyhow::Result;
use bincode::{config, Decode, Encode};
use entity::video::Model;

#[derive(Debug, Encode, Decode, Clone)]
pub enum NetData {
    Search(String),
    SearchResult(Vec<Model>),
    Remove(usize, String), // Index and id of the video
    Add(Model),
    Play,
    Pause,
    Next,
    SetVolume(f64),
    Move(usize, String, i32), // Index | Video id | Delta of the move
}

impl NetData {
    pub fn encode_message(&self) -> Result<Vec<u8>> {
        Ok(bincode::encode_to_vec(self, config::standard())?)
    }

    pub fn decode_message(bytes: &[u8]) -> Result<NetData> {
        Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
    }
}
