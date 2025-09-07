# gpa-rok

![gpa-rok logo](https://github.com/stevengpa/gpa-rok/blob/main/public/gpa-rok-with-label.png)

gpa-rok is a lightweight HTTP request forwarder built on a WebSocket pub/sub model. It runs a server that accepts inbound HTTP requests and broadcasts their metadata over WebSocket to connected clients. Each client can selectively forward those requests to a target HTTP service, apply header filters or inject headers, and report the forwarding result back to the server.

Purpose of gpa-rok
- Enable easy, dynamic request forwarding from a central HTTP ingress to one or more remote environments.
- Let clients “subscribe” to requests via WebSocket and decide which requests to forward based on the URL tail (path suffix) filters.
- Provide a minimal, configurable bridge for testing, debugging, or mirroring traffic without heavy infrastructure.

Key components
- Server (bin: gpa-rok-server):
  - Exposes an HTTP endpoint and a WebSocket endpoint.
  - When an HTTP request is received, it broadcasts a JSON representation of the request to all connected clients.
- Client (bin: gpa-rok-client):
  - Connects to the server via WebSocket and listens for broadcasted requests.
  - Optionally filters which requests to handle using listen_tails.
  - Forwards the request to a configured target, applying header filtering/injection.

Quick start
1) Build
- Rust toolchain required. From the project root:
  - cargo build --release

2) Run the server
- Configure server_config.json as needed, then:
  - RUST_LOG=info cargo run --bin gpa-rok-server
- The server starts:
  - WebSocket: ws://<host>:<port>/socket (from server_config.json)
  - HTTP: http://<host>:<port> (from server_config.json)

3) Run a client
- Configure client_config.json (sample included). Then:
  - RUST_LOG=info cargo run --bin gpa-rok-client
- The client connects to the server’s WebSocket and waits for request broadcasts.

Configuration overview
- server_config.json: Defines ws_server and http_server addresses. The HTTP server accepts requests and triggers broadcasts; clients receive those and may forward.
- client_config.json:
  - ws_server.host/port: Server to connect to via WebSocket.
  - ws_server.listen_tails: List of path tails to accept ("*" to accept all). If non-empty and not containing "*", only listed tails are forwarded.
  - target: Base URL to which requests get forwarded.
  - forward_host: Host header to use when forwarding (host + port).
  - headers: Extra headers to add to forwarded requests (name/value pairs).
  - strip_headers: Headers removed from the original before forwarding.

Binaries (optional prebuilt)
- See binaries/ for example prebuilt client and server binaries for selected platforms (e.g., mac-arm, ubuntu) and versions.

Logging
- Controlled via env_logger: set RUST_LOG=debug|info|warn|error to adjust verbosity.

License
- MIT License. See the LICENSE file: https://github.com/stevengpa/gpa-rok/blob/main/LICENSE
