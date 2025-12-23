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
    headless_running: bool,
    node_child_id: Option<u32>,
    miner_child_id: Option<u32>,
    headless_child_id: Option<u32>,
    explorer_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    data_dir: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_running: false,
            miner_running: false,
            explorer_server_running: false,
            headless_running: false,
            node_child_id: None,
            miner_child_id: None,
            headless_child_id: None,
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
pub struct HeadlessConfig {
    pub port: u16,
    pub fullnode_url: String,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            port: 8001,
            fullnode_url: "http://localhost:8080/v1a/".to_string(),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAddress {
    pub address: String,
    pub index: u32,
    pub balance: Option<u64>, // Balance in HTR cents (1 HTR = 100 cents)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendTxRequest {
    pub address: String,
    pub amount: u64, // Amount in HTR cents
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullnodeBalance {
    pub available: i64,
    pub locked: i64,
}

// Headless wallet structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeadlessWallet {
    pub wallet_id: String,
    pub status: String,
    pub status_code: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateHeadlessWalletRequest {
    pub wallet_id: String,
    pub seed: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessWalletBalance {
    pub available: u64,
    pub locked: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessWalletSendTxRequest {
    pub wallet_id: String,
    pub address: String,
    pub amount: u64,
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

// Get the path to the wallet-headless-dist directory
fn get_headless_dist_path() -> std::path::PathBuf {
    // In dev mode, wallet-headless-dist is in src-tauri/wallet-headless-dist/
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wallet-headless-dist");
    if dev_path.exists() {
        return dev_path;
    }

    // Fallback to current dir
    std::path::PathBuf::from("wallet-headless-dist")
}

// Generate wallet-headless config file in the dist directory
fn generate_headless_config(config: &HeadlessConfig, headless_dist_path: &std::path::Path) -> Result<(), String> {
    // wallet-headless expects config.js in the dist directory (hardcoded as ./config.js)
    let config_path = headless_dist_path.join("dist").join("config.js");

    // Generate config.js for wallet-headless
    // txMiningUrl is required for privatenet - point to local fullnode's mining endpoint
    let config_content = format!(
        r#"module.exports = {{
  http_bind_address: 'localhost',
  http_port: {},
  network: 'privatenet',
  server: '{}',
  txMiningUrl: 'http://localhost:8080/v1a/',
  seeds: {{}},
  allowPassphrase: false,
  confirmFirstAddress: false,
  tokenUid: '00',
  gapLimit: 20,
  connectionTimeout: 5000,
}}
"#,
        config.port, config.fullnode_url
    );

    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write headless config: {}", e))?;

    Ok(())
}

// Kill any process using a specific port (cleanup from previous runs)
fn kill_process_on_port(port: u16) {
    #[cfg(unix)]
    {
        use std::process::Command;
        // Find and kill process using the port
        if let Ok(output) = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()
        {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.lines() {
                if let Ok(pid_num) = pid.trim().parse::<u32>() {
                    let _ = Command::new("kill").args(["-9", &pid_num.to_string()]).output();
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        // On Windows, use netstat to find the PID and taskkill to kill it
        if let Ok(output) = Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
                    if let Some(pid) = line.split_whitespace().last() {
                        let _ = Command::new("taskkill")
                            .args(["/PID", pid, "/F"])
                            .output();
                    }
                }
            }
        }
    }
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

    // Kill any zombie processes from previous runs
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    kill_process_on_port(8001); // wallet-headless port
    // Give the OS a moment to release the ports
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
        "headless_running": state_guard.headless_running,
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

// Get wallet addresses with balances
#[tauri::command]
async fn get_wallet_addresses(state: tauri::State<'_, SharedState>) -> Result<Vec<WalletAddress>, String> {
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    // Get current address from the wallet
    let address_response = client
        .get("http://127.0.0.1:8080/v1a/wallet/address")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch address: {}", e))?;

    let address_json: serde_json::Value = address_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse address response: {}", e))?;

    let current_address = address_json["address"]
        .as_str()
        .ok_or("Invalid address format")?
        .to_string();

    // Get wallet balance
    let balance_response = client
        .get("http://127.0.0.1:8080/v1a/wallet/balance")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch balance: {}", e))?;

    let balance_json: serde_json::Value = balance_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse balance response: {}", e))?;

    let balance = balance_json["balance"]["available"].as_u64();

    // Return the current address with its balance
    let wallet_addresses = vec![WalletAddress {
        address: current_address,
        index: 0,
        balance,
    }];

    Ok(wallet_addresses)
}

// Get fullnode wallet balance
#[tauri::command]
async fn get_fullnode_balance(
    state: tauri::State<'_, SharedState>,
) -> Result<FullnodeBalance, String> {
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://127.0.0.1:8080/v1a/wallet/balance/")
        .send()
        .await
        .map_err(|e| format!("Failed to get balance: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let result: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, response_text))?;

    if result["success"].as_bool().unwrap_or(false) {
        let balance = &result["balance"];
        Ok(FullnodeBalance {
            available: balance["available"].as_i64().unwrap_or(0),
            locked: balance["locked"].as_i64().unwrap_or(0),
        })
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to get balance: {}", message))
    }
}

// Send HTR to an address (faucet)
#[tauri::command]
async fn send_tx(
    state: tauri::State<'_, SharedState>,
    request: SendTxRequest,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    // Use the fullnode's wallet send_tokens endpoint
    let response = client
        .post("http://127.0.0.1:8080/v1a/wallet/send_tokens/")
        .json(&serde_json::json!({
            "data": {
                "inputs": [],
                "outputs": [{
                    "address": request.address,
                    "value": request.amount,
                }]
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send transaction: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let result: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, response_text))?;

    if result["success"].as_bool().unwrap_or(false) {
        let tx_hash = result["hash"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        Ok(format!("Transaction sent! Hash: {}", tx_hash))
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Transaction failed: {}", message))
    }
}

// Start the wallet-headless service
#[tauri::command]
async fn start_headless(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<HeadlessConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    let mut state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting wallet-headless".to_string());
    }

    if state_guard.headless_running {
        return Err("Wallet-headless is already running".to_string());
    }

    let headless_path = get_headless_dist_path();
    if !headless_path.exists() {
        return Err(format!(
            "Wallet-headless dist not found at {:?}. Run 'build-wallet-headless' first.",
            headless_path
        ));
    }

    // Kill any zombie process on the headless port
    kill_process_on_port(config.port);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Generate config file in the dist directory
    generate_headless_config(&config, &headless_path)?;

    // Find node binary to run with
    let entry_point = headless_path.join("dist").join("index.js");
    let working_dir = headless_path.join("dist");

    // Spawn the process using node (working dir must be dist/ where config.js is)
    let mut child = TokioCommand::new("node")
        .args([entry_point.to_string_lossy().as_ref()])
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn wallet-headless: {}", e))?;

    let pid = child.id().unwrap_or(0);
    state_guard.headless_running = true;
    state_guard.headless_child_id = Some(pid);

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
                let _ = app_handle.emit("headless-log", &line);
            }
        });
    }

    // Spawn task for stderr
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle2.emit("headless-log", &line);
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
            state_guard.headless_running = false;
            state_guard.headless_child_id = None;
        }

        let _ = app_handle3.emit("headless-terminated", code);
    });

    Ok(format!("Wallet-headless started on port {}", config.port))
}

// Stop the wallet-headless service
#[tauri::command]
async fn stop_headless(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    // Kill the process
    if let Some(pid) = state_guard.headless_child_id {
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

    state_guard.headless_running = false;
    state_guard.headless_child_id = None;

    Ok("Wallet-headless stopped".to_string())
}

// Get headless status
#[tauri::command]
async fn get_headless_status(state: tauri::State<'_, SharedState>) -> Result<HeadlessStatus, String> {
    let state_guard = state.lock().await;

    Ok(HeadlessStatus {
        running: state_guard.headless_running,
        port: if state_guard.headless_running { Some(8001) } else { None },
    })
}

// Generate a new BIP39 seed phrase (24 words)
#[tauri::command]
async fn generate_seed() -> Result<String, String> {
    use bip39::{Language, Mnemonic};

    // Generate 32 bytes of entropy for 24 words
    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| format!("Failed to generate random bytes: {}", e))?;

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}

// Create a new wallet via wallet-headless
#[tauri::command]
async fn create_headless_wallet(
    state: tauri::State<'_, SharedState>,
    request: CreateHeadlessWalletRequest,
) -> Result<HeadlessWallet, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    // Start a wallet with the provided seed
    let response = client
        .post("http://localhost:8001/start")
        .json(&serde_json::json!({
            "wallet-id": request.wallet_id,
            "seed": request.seed,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create wallet: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(HeadlessWallet {
            wallet_id: request.wallet_id,
            status: "starting".to_string(),
            status_code: None,
        })
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to create wallet: {}", message))
    }
}

// Get wallet status from headless
#[tauri::command]
async fn get_headless_wallet_status(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<HeadlessWallet, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/status")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet status: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let status_code = result["statusCode"].as_i64().map(|c| c as i32);
    let status_message = result["statusMessage"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    Ok(HeadlessWallet {
        wallet_id,
        status: status_message,
        status_code,
    })
}

// Get wallet balance from headless
#[tauri::command]
async fn get_headless_wallet_balance(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<HeadlessWalletBalance, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/balance")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet balance: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let available = result["available"].as_u64().unwrap_or(0);
    let locked = result["locked"].as_u64().unwrap_or(0);

    Ok(HeadlessWalletBalance { available, locked })
}

// Get wallet addresses from headless
#[tauri::command]
async fn get_headless_wallet_addresses(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<Vec<String>, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/addresses")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet addresses: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let addresses = result["addresses"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(addresses)
}

// Send transaction from headless wallet
#[tauri::command]
async fn headless_wallet_send_tx(
    state: tauri::State<'_, SharedState>,
    request: HeadlessWalletSendTxRequest,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/simple-send-tx")
        .header("X-Wallet-Id", &request.wallet_id)
        .json(&serde_json::json!({
            "address": request.address,
            "value": request.amount,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send transaction: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let result: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, response_text))?;

    if result["success"].as_bool().unwrap_or(false) {
        let tx_hash = result["hash"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        Ok(format!("Transaction sent! Hash: {}", tx_hash))
    } else {
        // Try multiple error message locations
        let message = result["message"]
            .as_str()
            .or_else(|| result["error"].as_str())
            .unwrap_or(&response_text)
            .to_string();
        Err(format!("Transaction failed: {}", message))
    }
}

// Close a headless wallet
#[tauri::command]
async fn close_headless_wallet(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/stop")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to close wallet: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(format!("Wallet '{}' closed", wallet_id))
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to close wallet: {}", message))
    }
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
            get_wallet_addresses,
            get_fullnode_balance,
            send_tx,
            start_explorer_server,
            stop_explorer_server,
            start_headless,
            stop_headless,
            get_headless_status,
            generate_seed,
            create_headless_wallet,
            get_headless_wallet_status,
            get_headless_wallet_balance,
            get_headless_wallet_addresses,
            headless_wallet_send_tx,
            close_headless_wallet,
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

                if let Some(pid) = state.headless_child_id {
                    eprintln!("Cleaning up wallet-headless process (PID: {})", pid);
                    kill_process(pid);
                }

                if let Some(pid) = state.node_child_id {
                    eprintln!("Cleaning up node process (PID: {})", pid);
                    kill_process(pid);
                }
            }
        });
}
