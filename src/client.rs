// Run: RUST_LOG=debug cargo run --bin gpa-rok-server
mod entities;
use crate::entities::HttpConfig;

use log::{debug, info};
use env_logger;
use std::fs;

use tokio_tungstenite::{
    tungstenite::{
        connect,
        Message,
    },
};

fn client(host: &str, port: &str) {
    let ws_address = format!("ws://{}:{}/socket", host, port);
    let (mut socket, response) = connect(&ws_address).expect("Can't connect to the server");

    info!("Connected to the server {}", &ws_address);

    debug!("Response contains the following headers:");
    for (ref header, _value) in response.headers() {
        debug!("* {}: {:?}", header, _value);
    }

    socket.send(Message::Text("Hello Websocket".into())).unwrap();
    loop {
        let msg = socket.read().expect("Error reading message from socket");
        info!("Received Message: {:?}", msg);
    }
}

#[tokio::main]
async fn main () {
    env_logger::init();

    env_logger::init();

    let config_path = "client_config.json".to_owned();
    let config_info = fs::read_to_string(&config_path).expect("Failed to read config file");
    let config: HttpConfig = serde_json::from_str(&config_info).expect("Failed to parse config file");

    client(&config.ws_server.host, &config.ws_server.port.to_string());
}