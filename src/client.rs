// RUST_LOG=debug cargo run --bin gpa-rok-client
mod config_domain;
mod client_domain;
mod server_domain;
use crate::config_domain::HttpConfig;
use crate::client_domain::{ClientMessageCategory, ClientMessage};
use crate::server_domain::{ServerMessageCategory, ServerMessage};

use std::collections::{HashMap, HashSet};
use log::{debug, info, error};
use env_logger;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use reqwest::header;

#[derive(Serialize, Deserialize, Debug)]
struct HttpRequestReply {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    query: HashMap<String, String>,
    body: String,
    tail: String,
}

#[tokio::main]
async fn main () {
    env_logger::init();

    let config_path = "client_config.json".to_owned();
    let config_info = fs::read_to_string(&config_path).expect("Failed to read config file");
    let config: HttpConfig = serde_json::from_str(&config_info).expect("Failed to parse config file");

    client(config).await;
}

async fn client(client_config: HttpConfig) {
    let ws_address = format!("ws://{}:{}/socket", client_config.ws_server.host.clone(), client_config.ws_server.port.clone());
    let (ws_stream, response) = connect_async(&ws_address)
        .await
        .expect("Can't connect to the server");

    let (mut write, mut read) = ws_stream.split();

    info!("Connected to the server {}", &ws_address);

    debug!("Response contains the following headers:");
    for (ref header, _value) in response.headers() {
        debug!("* {}: {:?}", header, _value);
    }

    let joined_message = ClientMessage::new("New client joined", ClientMessageCategory::ClientConnected);
    let message_for_server = serde_json::to_string(&joined_message).unwrap();

    write.send(message_for_server.into()).await.unwrap();

    let write_arc = Arc::new(Mutex::new(write));

    // Spawn a task to handle incoming messages
    let write_for_task = write_arc.clone();
    let config_clone = client_config.clone();

    loop {
        let msg = read.next().await.expect("Error reading message from socket").unwrap();

        if let Message::Text(json) = msg.clone() {
            let server_message = serde_json::from_str::<ServerMessage>(&json).unwrap();

            match server_message.category {
                ServerMessageCategory::ForwardHttpRequest => {
                    match serde_json::from_str::<HttpRequestReply>(&server_message.message) {
                        Ok(req) => {
                            let write_clone = write_for_task.clone();
                            let client_http_config = config_clone.clone();

                            // Skip forward request if tail is not in listen tails
                            let req_tail = req.tail.clone();
                            let listen_tails = config_clone.ws_server.listen_tails.clone();

                            let has_empty_tails = listen_tails.is_empty();
                            let has_tail_filters = !listen_tails.contains(&"*".to_string()) && listen_tails.len() > 0;
                            let is_listed_coming_tail = listen_tails.contains(&req_tail);

                            if  has_empty_tails || (has_tail_filters && !is_listed_coming_tail) {
                                debug!("Request tail '{}' is not in the listening tails list. Forward request was skipped!", req_tail);
                                continue;
                            }

                            tokio::spawn(async move {
                                match forward_request(client_http_config, req).await {
                                    Ok(result) => {
                                        let message_result = ClientMessage::new(result.as_str(), ClientMessageCategory::ClientForwardedResult);
                                        let message_for_server = serde_json::to_string(&message_result).unwrap();

                                        let mut sender = write_clone.lock().await;
                                        sender.send(message_for_server.into()).await.unwrap();
                                    }
                                    Err(e) => {
                                        error!("Failed to forward request: {}", e);
                                    }
                                }
                            });
                        }
                        _ => info!("Message received: {}", msg),
                    }
                }
                ServerMessageCategory::AnnounceClientConnected => {
                    info!("{}", server_message.message);
                }
            }
        }
    }
}

async fn forward_request(config: HttpConfig, req: HttpRequestReply) -> Result<String, reqwest::Error> {
    let req_client = reqwest::Client::new();
    let mut url = format!("{}", config.target);

    // Append query params if any
    if !req.query.is_empty() {
        let query_str: Vec<String> = req.query.iter().map(|(k,v)| format!("{}={}", k, v)).collect();
        url.push('?');
        url.push_str(&query_str.join("&"));
    }

    // Filter headers
    let strip: HashSet<String> = config.strip_headers.iter().map(|h| h.to_lowercase()).collect();
    let headers: Vec<(String, String)> = req
        .headers
        .into_iter()
        .filter(|(k, _)| !strip.contains(&k.to_lowercase()))
        .collect();

    // Create request builder
    let mut builder = req_client.request(req.method.parse().unwrap(), &url);

    // Add allowed forwarded headers
    for (k, v) in headers {
        builder = builder.header(&k, &v);
    }

    // Add config headers
    for header in config.headers {
        if !&header.name.is_empty() && !&header.value.is_empty() {
            builder = builder.header(&header.name, &header.value);
        }
    }

    // Add host header
    let host = format!("{}:{}", config.forward_host.host, config.forward_host.port);
    builder = builder.header(header::HOST, host.as_str());

    // Add body
    if !req.body.is_empty() {
        builder = builder.body(req.body.clone());
    }

    // Send request
    let response = builder.send().await;
    let status = response?.status();

    let result = format!("Forwarded request to {} {} - Status: {}", req.method.to_string(), url, status);
    info!("{}", result.as_str());

    Ok(result)
}