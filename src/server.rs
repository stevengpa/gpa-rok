// RUST_LOG=debug cargo run --bin gpa-rok-server
// podman-compose -f composer.server.yml down
// podman-compose -f composer.server.yml up --build
use env_logger;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};
use log::{info, debug};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use futures_util::{SinkExt, StreamExt};
use warp::Filter;
use warp::filters::BoxedFilter;
use warp::Reply;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use warp::http::{HeaderMap, Method};
use warp::path::FullPath;

mod config_domain;
mod server_domain;
mod client_domain;

use crate::config_domain::WsConfig;
use crate::server_domain::{ServerMessageCategory, ServerMessage};
use crate::client_domain::{ClientMessageCategory, ClientMessage};

#[derive(Serialize, Deserialize, Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    query: HashMap<String, String>,
    body: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let config_path = "server_config.json".to_owned();
    let config_info = fs::read_to_string(&config_path).expect("Failed to read config file");
    let config: WsConfig = serde_json::from_str(&config_info).expect("Failed to parse config file");

    let (tx, _) = broadcast::channel::<ServerMessage>(100);
    let tx_ws = tx.clone();

    // WebSocket Server
    let config_for_ws = config.clone();
    let ws_server = tokio::spawn(async move {
        server(config_for_ws.ws_server.host.as_str(), config_for_ws.ws_server.port.to_string().as_str(), &tx_ws).await;
    });

    // Http Server
    let tx_http = tx.clone();
    let http_server = {
        let send_route = http_send_route(tx_http);

        let ip: IpAddr = config.http_server.host.as_str().parse().expect("Invalid IP address");
        let port: u16 = config.http_server.port;
        let addr = SocketAddr::new(ip, port);

        info!("Http server starting at ........ http://{}:{}/send", &config.http_server.host, config.http_server.port);
        warp::serve(send_route.await).run(addr)
    };

    let _ = tokio::join!(ws_server, http_server);
}

async fn http_send_route(tx: broadcast::Sender<ServerMessage>) -> BoxedFilter<(impl Reply,)> {
    let tx_clone = tx.clone();

    // HTTP Server
    warp::any()
        .and(warp::path("send"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::method())
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(move |query: HashMap<String, String>, method: Method, path: FullPath, headers: HeaderMap, body: bytes::Bytes| {
            let method_str = method.to_string();
            let path_str = path.as_str().to_owned();

            let headers_vec: Vec<(String, String)> = headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let payload = HttpRequest {
                method: method_str,
                path: path_str,
                query,
                headers: headers_vec,
                body: String::from_utf8(body.to_vec()).unwrap_or_default(),
            };

            if let Ok(json) = serde_json::to_string(&payload) {
                let payload_str = serde_json::to_string(&payload).unwrap();
                let message_for_client = ServerMessage::new(&payload_str, ServerMessageCategory::ForwardHttpRequest);

                info!("Broadcasting request metadata to clients");
                let _ = tx_clone.send(message_for_client);
            }

            warp::reply::json(&serde_json::json!({ "status": "broadcasted" }))
        })
        .boxed()
}

async fn server(host: &str, port: &str, tx: &broadcast::Sender<ServerMessage>) {
    let address = format!("{}:{}", host, port);
    info!("Websocket server starting at ... ws://{}/socket", &address);

    let server = TcpListener::bind(address).await.unwrap();

    while let Ok((stream, _)) = server.accept().await {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            accept_connection(stream, tx_clone).await;
        });
    }
}

async fn accept_connection(stream: TcpStream, tx: broadcast::Sender<ServerMessage>) {
    let mut rx = tx.subscribe();

    let callback = |req: &Request, response: Response| {
        debug!("New WS connection: {}", req.uri().path());
        Ok(response)
    };

    let ws_stream = accept_hdr_async(stream, callback)
        .await
        .expect("WebSocket handshake failed");

    // Split stream into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Forward broadcast messages to this client
    // TX Receiver
    let forward_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let message_for_clients = serde_json::to_string(&msg).unwrap();

            if ws_sender.send(message_for_clients.into()).await.is_err() {
                break; // client disconnected
            }
        }
    });

    // Handle messages from client
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if msg.is_text() || msg.is_binary() {
            if let Message::Text(message) = msg.clone() {
                let client_message: ClientMessage = serde_json::from_str::<ClientMessage>(&message).unwrap();

                match client_message.category {
                    ClientMessageCategory::ClientForwardedResult => {
                        info!("{}", &client_message.message);
                    }
                    ClientMessageCategory::ClientConnected => {
                        info!("{}", &client_message.message);

                        let message_for_client = ServerMessage::new("New client joined", ServerMessageCategory::AnnounceClientConnected);
                        let _ = tx.send(message_for_client);
                    }
                }

            }
        }
    }

    forward_task.await.unwrap();
}
