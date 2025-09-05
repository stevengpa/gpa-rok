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
use serde::Serialize;
use futures_util::{SinkExt, StreamExt};
use warp::Filter;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use warp::http::{HeaderMap, Method};
use warp::path::FullPath;

mod entities;
use crate::entities::WsConfig;

#[derive(Serialize)]
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

    let (tx, _) = broadcast::channel::<String>(100);
    let tx_ws = tx.clone();

    // WebSocket Server
    let ws_server = tokio::spawn(async move {
        info!("WS server starting");
        server(&config.ws_server.host, config.ws_server.port.to_string().as_str(), &tx_ws).await;
    });

    let ip: IpAddr = config.http_server.host.as_str().parse().expect("Invalid IP address");
    let port: u16 = config.http_server.port;
    let addr = SocketAddr::new(ip, port);
    // HTTP Server
    let send_route = warp::any()
        .and(warp::path("send"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::method())
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(move |query: HashMap<String, String>, method: Method, path: FullPath, headers: HeaderMap, body: bytes::Bytes| {
            let method_str = method.to_string();
            let path_str = path.as_str().to_string();

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
                info!("Broadcasting request metadata to client");
                let tx_clone = tx.clone();
                let _ = tx_clone.send(json);
            }

            warp::reply::json(&serde_json::json!({ "status": "broadcasted" }))
        });

    let http_server = {
        info!("Http server starting");
        warp::serve(send_route).run(addr)
    };

    let _ = tokio::join!(ws_server, http_server);
}

async fn server(host: &str, port: &str, tx: &broadcast::Sender<String>) {
    let address = format!("{}:{}", host, port);
    info!("Starting server {}", &address);

    let server = TcpListener::bind(address).await.unwrap();

    while let Ok((stream, _)) = server.accept().await {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            accept_connection(stream, tx_clone).await;
        });
    }
}

async fn accept_connection(stream: TcpStream, tx: broadcast::Sender<String>) {
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
    let forward_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg.to_string().into())).await.is_err() {
                break; // client disconnected
            }
        }
    });

    // Handle messages from client
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if msg.is_text() || msg.is_binary() {
            debug!("Received from client: {:?}", &msg);
            let _ = tx.send(msg.to_string()); // broadcast to others
        }
    }

    forward_task.await.unwrap();
}
