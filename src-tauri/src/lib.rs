use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Request};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

// Application state
pub struct AppState {
    node_running: bool,
    miner_running: bool,
    explorer_server_running: bool,
    node_child_id: Option<u32>,
    miner_child_id: Option<u32>,
    explorer_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    data_dir: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_running: false,
            miner_running: false,
            explorer_server_running: false,
            node_child_id: None,
            miner_child_id: None,
            explorer_shutdown: None,
            data_dir: None,
        }
    }
}

type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub api_port: u16,
    pub stratum_port: u16,
    pub data_dir: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        // Use a directory in the user's home folder
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hathor-forge")
            .join("data");
        Self {
            api_port: 8080,
            stratum_port: 8000,
            data_dir: data_dir.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinerConfig {
    pub stratum_port: u16,
    pub address: String,
    pub threads: u32,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            stratum_port: 8000,
            address: "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb".to_string(), // Default localnet address (from HD wallet)
            threads: 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub block_height: Option<u64>,
    pub hash_rate: Option<f64>,
    pub peer_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinerStatus {
    pub running: bool,
    pub hash_rate: Option<f64>,
}

// Get the path to a binary (handles dev vs production)
fn get_binary_path(name: &str) -> std::path::PathBuf {
    // In dev mode, binaries are in src-tauri/binaries/
    // Get the target triple
    let target = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    } else {
        "x86_64-pc-windows-msvc"
    };

    let binaries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");

    // hathor-core uses onedir mode (folder with binary inside)
    if name == "hathor-core" {
        let onedir_path = binaries_dir
            .join(format!("{}-{}", name, target))
            .join(name);
        if onedir_path.exists() {
            return onedir_path;
        }
    }

    // For single-file binaries (cpuminer)
    let dev_path = binaries_dir.join(format!("{}-{}", name, target));
    if dev_path.exists() {
        return dev_path;
    }

    // Fallback to current dir
    std::path::PathBuf::from("binaries").join(format!("{}-{}", name, target))
}

// Start the Hathor fullnode
#[tauri::command]
async fn start_node(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<NodeConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    let mut state_guard = state.lock().await;

    if state_guard.node_running {
        return Err("Node is already running".to_string());
    }

    let binary_path = get_binary_path("hathor-core");

    // Ensure data directory exists
    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    // Development HD wallet seed (DO NOT use in production!)
    // This is a fixed seed for local development only
    let dev_wallet_words = "avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level";

    // Set DYLD_FALLBACK_LIBRARY_PATH for macOS to find bundled libraries
    // This prevents the "loading libcrypto in an unsafe way" abort
    let internal_dir = binary_path.parent().unwrap().join("_internal");

    // Spawn the process using tokio
    let mut child = TokioCommand::new(&binary_path)
        .env("DYLD_FALLBACK_LIBRARY_PATH", &internal_dir)
        .args([
            "run_node",
            "--localnet",
            "--status",
            &config.api_port.to_string(),
            "--stratum",
            &config.stratum_port.to_string(),
            "--data",
            &config.data_dir,
            "--wallet",
            "hd",
            "--words",
            dev_wallet_words,
            "--wallet-enable-api",
            "--wallet-index",
            "--allow-mining-without-peers",
            "--test-mode-tx-weight",
            "--unsafe-mode",
            "privatenet",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn hathor-core at {:?}: {}", binary_path, e))?;

    let pid = child.id().unwrap_or(0);
    state_guard.node_running = true;
    state_guard.node_child_id = Some(pid);
    state_guard.data_dir = Some(config.data_dir.clone());

    // Handle stdout
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_handle = app.clone();
    let app_handle2 = app.clone();

    // Spawn task for stdout
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit("node-log", &line);
            }
        });
    }

    // Spawn task for stderr (hathor-core sends all logs here)
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // hathor-core sends info/warning/error logs to stderr
                // Route them appropriately based on content
                let _ = app_handle2.emit("node-log", &line);
            }
        });
    }

    // Spawn task to wait for process termination and reset state
    let app_handle3 = app.clone();
    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.map(|s| s.code()).ok().flatten();

        // Reset state when process terminates
        {
            let mut state_guard = state_clone.lock().await;
            state_guard.node_running = false;
            state_guard.node_child_id = None;
        }

        let _ = app_handle3.emit("node-terminated", code);
    });

    Ok(format!("Node started on port {}", config.api_port))
}

// Stop the Hathor fullnode
#[tauri::command]
async fn stop_node(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }

    // Kill the process
    if let Some(pid) = state_guard.node_child_id {
        #[cfg(unix)]
        {
            use std::process::Command;
            // Send SIGTERM for graceful shutdown
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    state_guard.node_running = false;
    state_guard.node_child_id = None;

    Ok("Node stopped".to_string())
}

// Start the CPU miner
#[tauri::command]
async fn start_miner(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<MinerConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    let mut state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting miner".to_string());
    }

    if state_guard.miner_running {
        return Err("Miner is already running".to_string());
    }

    let binary_path = get_binary_path("cpuminer");

    // Spawn the process using tokio
    let mut child = TokioCommand::new(&binary_path)
        .args([
            "--algo",
            "sha256d",
            "--url",
            &format!("stratum+tcp://127.0.0.1:{}", config.stratum_port),
            "--coinbase-addr",
            &config.address,
            "--threads",
            &config.threads.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn cpuminer at {:?}: {}", binary_path, e))?;

    let pid = child.id().unwrap_or(0);
    state_guard.miner_running = true;
    state_guard.miner_child_id = Some(pid);

    // Handle stdout
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_handle = app.clone();
    let app_handle2 = app.clone();

    // Spawn task for stdout
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit("miner-log", &line);
            }
        });
    }

    // Spawn task for stderr (cpuminer outputs stats here)
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle2.emit("miner-stats", &line);
            }
        });
    }

    // Spawn task to wait for process termination and reset state
    let app_handle3 = app.clone();
    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.map(|s| s.code()).ok().flatten();

        // Reset state when process terminates
        {
            let mut state_guard = state_clone.lock().await;
            state_guard.miner_running = false;
            state_guard.miner_child_id = None;
        }

        let _ = app_handle3.emit("miner-terminated", code);
    });

    Ok(format!("Miner started with {} threads", config.threads))
}

// Stop the CPU miner
#[tauri::command]
async fn stop_miner(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.miner_running {
        return Err("Miner is not running".to_string());
    }

    // Kill the process
    if let Some(pid) = state_guard.miner_child_id {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    state_guard.miner_running = false;
    state_guard.miner_child_id = None;

    Ok("Miner stopped".to_string())
}

// Get node status from the API
#[tauri::command]
async fn get_node_status(state: tauri::State<'_, SharedState>) -> Result<NodeStatus, String> {
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Ok(NodeStatus {
            running: false,
            block_height: None,
            hash_rate: None,
            peer_count: None,
        });
    }

    // Try to fetch status from the node API
    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:8080/v1a/status").send().await {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                let block_height = json
                    .get("dag")
                    .and_then(|d| d.get("best_block"))
                    .and_then(|b| b.get("height"))
                    .and_then(|h| h.as_u64());

                Ok(NodeStatus {
                    running: true,
                    block_height,
                    hash_rate: None,
                    peer_count: Some(0), // Localnet has no peers
                })
            } else {
                Ok(NodeStatus {
                    running: true,
                    block_height: None,
                    hash_rate: None,
                    peer_count: None,
                })
            }
        }
        Err(_) => Ok(NodeStatus {
            running: true, // Process is running but API might not be ready
            block_height: None,
            hash_rate: None,
            peer_count: None,
        }),
    }
}

// Get miner status
#[tauri::command]
async fn get_miner_status(state: tauri::State<'_, SharedState>) -> Result<MinerStatus, String> {
    let state_guard = state.lock().await;

    Ok(MinerStatus {
        running: state_guard.miner_running,
        hash_rate: None, // TODO: Parse from miner output
    })
}

// Get current state
#[tauri::command]
async fn get_state(state: tauri::State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;

    Ok(serde_json::json!({
        "node_running": state_guard.node_running,
        "miner_running": state_guard.miner_running,
        "explorer_server_running": state_guard.explorer_server_running,
        "data_dir": state_guard.data_dir,
    }))
}

// Get the default data directory path
fn get_default_data_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("hathor-forge")
        .join("data")
}

// Reset blockchain data (removes the data directory)
#[tauri::command]
async fn reset_data(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let state_guard = state.lock().await;

    // Don't allow reset while node is running
    if state_guard.node_running {
        return Err("Cannot reset data while node is running. Stop the node first.".to_string());
    }

    // Use the stored data dir or default
    let data_dir = state_guard
        .data_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_default_data_dir);

    drop(state_guard); // Release lock before file operations

    if data_dir.exists() {
        fs::remove_dir_all(&data_dir)
            .map_err(|e| format!("Failed to remove data directory: {}", e))?;
    }

    Ok(format!("Data directory removed: {:?}", data_dir))
}

// Proxy HTTP requests to the fullnode
async fn proxy_api(Path(path): Path<String>, req: Request) -> Response {
    // Include query string if present
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let fullnode_url = format!("http://127.0.0.1:8080/v1a/{}{}", path, query);

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
                    .unwrap();
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
                        if let Ok(header_name) =
                            axum::http::HeaderName::try_from(name.as_str())
                        {
                            if let Ok(header_value) =
                                axum::http::HeaderValue::from_bytes(value.as_bytes())
                            {
                                builder = builder.header(header_name, header_value);
                            }
                        }
                    }

                    builder.body(Body::from(body.to_vec())).unwrap()
                }
                Err(_) => Response::builder()
                    .status(502)
                    .body(Body::from("Failed to read response from fullnode"))
                    .unwrap(),
            }
        }
        Err(e) => Response::builder()
            .status(502)
            .body(Body::from(format!("Failed to connect to fullnode: {}", e)))
            .unwrap(),
    }
}

// Proxy WebSocket connections to the fullnode
async fn proxy_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ws_proxy)
}

async fn handle_ws_proxy(mut client_ws: WebSocket) {
    // Connect to fullnode WebSocket
    let fullnode_url = "ws://127.0.0.1:8080/v1a/ws/";

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
                    if client_sink.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    if client_sink.send(Message::Binary(data.into())).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    if client_sink.send(Message::Ping(data.into())).await.is_err() {
                        break;
                    }
                }
                Ok(tungstenite::Message::Pong(data)) => {
                    if client_sink.send(Message::Pong(data.into())).await.is_err() {
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

// Get the path to the explorer-dist directory
fn get_explorer_dist_path() -> std::path::PathBuf {
    // In dev mode, explorer-dist is in src-tauri/explorer-dist/
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("explorer-dist");
    if dev_path.exists() {
        return dev_path;
    }

    // Fallback to current dir
    std::path::PathBuf::from("explorer-dist")
}

// Start the explorer HTTP server
#[tauri::command]
async fn start_explorer_server(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if state_guard.explorer_server_running {
        return Err("Explorer server is already running".to_string());
    }

    let explorer_path = get_explorer_dist_path();
    if !explorer_path.exists() {
        return Err(format!(
            "Explorer dist not found at {:?}. Run 'build-explorer' first.",
            explorer_path
        ));
    }

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

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
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    // Create the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port 3001: {}", e))?;

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

    Ok("Explorer server started on http://localhost:3001".to_string())
}

// Stop the explorer HTTP server
#[tauri::command]
async fn stop_explorer_server(state: tauri::State<'_, SharedState>) -> Result<String, String> {
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

// Helper function to kill a process by PID
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        use std::process::Command;
        // Send SIGTERM for graceful shutdown
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
        // Give it a moment, then force kill if needed
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(Mutex::new(AppState::default())) as SharedState;
    let cleanup_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            start_node,
            stop_node,
            start_miner,
            stop_miner,
            get_node_status,
            get_miner_status,
            get_state,
            reset_data,
            start_explorer_server,
            stop_explorer_server,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Cleanup: kill any running processes
                let state = cleanup_state.blocking_lock();

                if let Some(pid) = state.miner_child_id {
                    eprintln!("Cleaning up miner process (PID: {})", pid);
                    kill_process(pid);
                }

                if let Some(pid) = state.node_child_id {
                    eprintln!("Cleaning up node process (PID: {})", pid);
                    kill_process(pid);
                }
            }
        });
}
