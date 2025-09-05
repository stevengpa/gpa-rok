// use std::sync::Arc;
use serde::Deserialize;
//
// #[derive(Deserialize, Debug, Clone)]
// pub struct Server {
//     pub port: u16,
//     pub host: String,
// }
//
// #[derive(Deserialize, Debug, Clone)]
// pub struct Config {
//     pub server: Server,
//     pub targets: Vec<String>,
//     pub strip_headers: Vec<String>,
// }
//
// #[derive(Clone)]
// pub struct AppState {
//     pub config: Arc<Config>
// }

#[derive(Deserialize, Debug, Clone)]
pub struct ServerWS {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerHttp {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WsConfig {
    pub ws_server: ServerWS,
    pub http_server: ServerHttp,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HttpConfig {
    pub ws_server: ServerWS,
    pub target: String,
    pub strip_headers: String,
}