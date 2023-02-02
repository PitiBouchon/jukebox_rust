use anyhow::Result;
#[cfg(feature = "axum")]
use axum::extract::ws;
use bincode::{config, Decode, Encode};
use gloo_net::websocket;
use my_youtube_extractor::youtube_info::YtVideoPageInfo;

#[derive(Debug, Encode, Decode, Clone)]
pub enum NetDataAxum {
    Remove(String),
    Add(YtVideoPageInfo),
    Search(Vec<YtVideoPageInfo>),
}

#[derive(Debug, Encode, Decode)]
pub enum NetDataYew {
    Search(String),
    Remove(String),
    Add(YtVideoPageInfo),
}

impl NetDataYew {
    pub fn encode_yew_message(&self) -> Result<websocket::Message> {
        let encoded = bincode::encode_to_vec(self, config::standard())?;
        Ok(websocket::Message::Bytes(encoded))
    }

    pub fn decode_message(bytes: &[u8]) -> Result<NetDataYew> {
        Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
    }
}

impl NetDataAxum {
    #[cfg(feature = "axum")]
    pub fn encode_axum_message(&self) -> Result<ws::Message> {
        let encoded = bincode::encode_to_vec(self, config::standard())?;
        Ok(ws::Message::Binary(encoded))
    }

    pub fn decode_message(bytes: &[u8]) -> Result<NetDataAxum> {
        Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
    }
}
