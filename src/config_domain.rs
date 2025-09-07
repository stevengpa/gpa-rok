use serde::Deserialize;

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
pub struct ForwardHost {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerWSForHttp {
    pub host: String,
    pub port: u16,
    pub listen_tails: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HttpConfig {
    pub ws_server: ServerWSForHttp,
    pub target: String,
    pub forward_host: ForwardHost,
    pub headers: Vec<HttpHeader>,
    pub strip_headers: Vec<String>,
}
