use crate::*;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Path, Request};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tauri::Emitter;
use tokio_tungstenite::tungstenite;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

/// Newtype so Axum's Extension layer carries the fullnode port unambiguously.
#[derive(Clone, Copy)]
struct FullnodePort(u16);

// Get the path to the explorer-dist directory
fn get_explorer_dist_path() -> std::path::PathBuf {
    // In dev mode, explorer-dist is in src-tauri/explorer-dist/
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("explorer-dist");
    if dev_path.exists() {
        return dev_path;
    }

    // Production: Tauri places resources in Contents/Resources/ on macOS
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(target_os = "macos")]
            {
                let resources_dist = exe_dir.join("../Resources/explorer-dist");
                if resources_dist.exists() {
                    return resources_dist;
                }
            }

            let exe_dist = exe_dir.join("explorer-dist");
            if exe_dist.exists() {
                return exe_dist;
            }

            // Windows: some installers place resources in a resources/ subdirectory
            #[cfg(target_os = "windows")]
            {
                let resources_dist = exe_dir.join("resources").join("explorer-dist");
                if resources_dist.exists() {
                    return resources_dist;
                }
            }
        }
    }

    // Fallback
    std::path::PathBuf::from("explorer-dist")
}

// Proxy HTTP requests to the fullnode
async fn proxy_api(
    Extension(FullnodePort(port)): Extension<FullnodePort>,
    Path(path): Path<String>,
    req: Request,
) -> Response {
    // Include query string if present
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let fullnode_url = format!("http://127.0.0.1:{}/v1a/{}{}", port, path, query);

    let client = reqwest::Client::new();
    let method = req.method().clone();
    let headers = req.headers().clone();

    // Build the request to the fullnode
    let mut builder = match method.as_str() {
        "GET" => client.get(&fullnode_url),
        "POST" => client.post(&fullnode_url),
        "PUT" => client.put(&fullnode_url),
        "DELETE" => client.delete(&fullnode_url),
        "PATCH" => client.patch(&fullnode_url),
        _ => client.get(&fullnode_url),
    };

    // Forward headers (except host)
    for (name, value) in headers.iter() {
        if name != "host" {
            if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
                {
                    builder = builder.header(header_name, header_value);
                }
            }
        }
    }

    // Forward body for POST/PUT/PATCH
    if method == "POST" || method == "PUT" || method == "PATCH" {
        let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Response::builder()
                    .status(500)
                    .body(Body::from("Failed to read request body"))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(500)
                            .body(Body::from("Failed to build error response"))
                            .expect("hardcoded response")
                    });
            }
        };
        builder = builder.body(body_bytes.to_vec());
    }

    // Make the request
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            match response.bytes().await {
                Ok(body) => {
                    let mut builder = Response::builder().status(status.as_u16());

                    // Forward response headers
                    for (name, value) in headers.iter() {
                        if let Ok(header_name) = axum::http::HeaderName::try_from(name.as_str()) {
                            if let Ok(header_value) =
                                axum::http::HeaderValue::from_bytes(value.as_bytes())
                            {
                                builder = builder.header(header_name, header_value);
                            }
                        }
                    }

                    builder.body(Body::from(body.to_vec())).unwrap_or_else(|_| {
                        Response::builder()
                            .status(500)
                            .body(Body::from("Failed to build response"))
                            .expect("hardcoded response")
                    })
                }
                Err(_) => Response::builder()
                    .status(502)
                    .body(Body::from("Failed to read response from fullnode"))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(500)
                            .body(Body::from("Internal error"))
                            .expect("hardcoded response")
                    }),
            }
        }
        Err(e) => Response::builder()
            .status(502)
            .body(Body::from(format!("Failed to connect to fullnode: {}", e)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(500)
                    .body(Body::from("Internal error"))
                    .expect("hardcoded response")
            }),
    }
}

// Proxy WebSocket connections to the fullnode
async fn proxy_ws(
    Extension(FullnodePort(port)): Extension<FullnodePort>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_proxy(socket, port))
}

async fn handle_ws_proxy(mut client_ws: WebSocket, fullnode_port: u16) {
    // Connect to fullnode WebSocket
    let fullnode_url = &format!("ws://127.0.0.1:{}/v1a/ws/", fullnode_port);

    let ws_stream = match tokio_tungstenite::connect_async(fullnode_url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            let _ = client_ws
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1011,
                    reason: format!("Failed to connect to fullnode: {}", e).into(),
                })))
                .await;
            return;
        }
    };

    let (mut fullnode_sink, mut fullnode_stream) = ws_stream.split();
    let (mut client_sink, mut client_stream) = client_ws.split();

    // Forward messages from client to fullnode
    let client_to_fullnode = async {
        while let Some(msg) = client_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if fullnode_sink
                        .send(tungstenite::Message::Text(text.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Binary(data)) => {
                    if fullnode_sink
                        .send(tungstenite::Message::Binary(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    if fullnode_sink
                        .send(tungstenite::Message::Ping(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Pong(data)) => {
                    if fullnode_sink
                        .send(tungstenite::Message::Pong(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
            }
        }
    };

    // Forward messages from fullnode to client
    let fullnode_to_client = async {
        while let Some(msg) = fullnode_stream.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    if client_sink.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    if client_sink.send(Message::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    if client_sink.send(Message::Ping(data)).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Pong(data)) => {
                    if client_sink.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_fullnode => {},
        _ = fullnode_to_client => {},
    }
}

// Start the explorer HTTP server
#[tauri::command]
pub(crate) async fn start_explorer_server(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    // Check state early without holding lock across async bind
    let (explorer_port, fullnode_port) = {
        let state_guard = state.lock().await;
        if state_guard.explorer_server_running {
            return Err("Explorer server is already running".to_string());
        }
        (state_guard.ports.explorer, state_guard.ports.fullnode_api)
    };

    let explorer_path = get_explorer_dist_path();
    if !explorer_path.exists() {
        return Err(format!(
            "Explorer dist not found at {:?}. Run 'build-explorer' first.",
            explorer_path
        ));
    }

    // Build the router with CORS support and API proxy
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app_router = Router::new()
        // API proxy routes
        .route("/v1a/ws/", get(proxy_ws))
        .route("/v1a/*path", any(proxy_api))
        // Static files for explorer
        .fallback_service(ServeDir::new(&explorer_path).append_index_html_on_directories(true))
        .layer(Extension(FullnodePort(fullnode_port)))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], explorer_port));

    // Bind without holding the lock (async I/O)
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        format!("Failed to bind to port {}: {}", explorer_port, e)
    })?;

    // Re-acquire lock and re-check before committing state
    let mut state_guard = state.lock().await;
    if state_guard.explorer_server_running {
        return Err("Explorer server is already running".to_string());
    }

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    state_guard.explorer_server_running = true;
    state_guard.explorer_shutdown = Some(shutdown_tx);

    let app_handle = app.clone();
    let state_clone = state.inner().clone();

    // Spawn the server
    tokio::spawn(async move {
        let server = axum::serve(listener, app_router).with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });

        if let Err(e) = server.await {
            let _ = app_handle.emit("explorer-error", format!("Explorer server error: {}", e));
        }

        // Reset state when server stops
        {
            let mut state_guard = state_clone.lock().await;
            state_guard.explorer_server_running = false;
            state_guard.explorer_shutdown = None;
        }

        let _ = app_handle.emit("explorer-terminated", ());
    });

    Ok(format!(
        "Explorer server started on http://localhost:{}",
        crate::config::DEFAULT_EXPLORER_PORT
    ))
}

// Stop the explorer HTTP server
#[tauri::command]
pub(crate) async fn stop_explorer_server(
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.explorer_server_running {
        return Err("Explorer server is not running".to_string());
    }

    // Send shutdown signal
    if let Some(shutdown_tx) = state_guard.explorer_shutdown.take() {
        let _ = shutdown_tx.send(());
    }

    state_guard.explorer_server_running = false;

    Ok("Explorer server stopped".to_string())
}
