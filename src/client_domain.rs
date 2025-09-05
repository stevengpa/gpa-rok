use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessageCategory {
    ClientForwardedResult,
    ClientConnected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientMessage {
    pub message: String,
    pub category: ClientMessageCategory,
}

impl ClientMessage {
    pub fn new(message: &str, category: ClientMessageCategory) -> Self {
        ClientMessage {
            message: message.to_owned(),
            category
        }
    }
}