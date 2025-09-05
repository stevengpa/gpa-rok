use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessageCategory {
    ForwardHttpRequest,
    AnnounceClientConnected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerMessage {
    pub message: String,
    pub category: ServerMessageCategory,
}

impl ServerMessage {
    pub fn new(message: &str, category: ServerMessageCategory) -> Self {
        ServerMessage {
            message: message.to_owned(),
            category
        }
    }
}
