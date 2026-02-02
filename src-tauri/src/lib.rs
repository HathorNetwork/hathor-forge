use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

pub mod mcp;
pub mod tui;

/// Ring buffer for service log lines, shared across all service processes.
const LOG_BUFFER_CAPACITY: usize = 1000;

#[derive(Clone)]
pub struct LogBuffer(Arc<std::sync::Mutex<VecDeque<String>>>);

impl LogBuffer {
    pub fn new() -> Self {
        Self(Arc::new(std::sync::Mutex::new(VecDeque::with_capacity(
            LOG_BUFFER_CAPACITY,
        ))))
    }

    pub fn push(&self, line: String) {
        let mut buf = self.0.lock().unwrap();
        if buf.len() == LOG_BUFFER_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(line);
    }

    pub fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().iter().cloned().collect()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// Application state
pub struct AppState {
    pub node_running: bool,
    pub miner_running: bool,
    pub explorer_server_running: bool,
    pub headless_running: bool,
    pub tx_mining_running: bool,
    pub node_child_id: Option<u32>,
    pub miner_child_id: Option<u32>,
    pub headless_child_id: Option<u32>,
    pub tx_mining_child_id: Option<u32>,
    pub explorer_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub data_dir: Option<String>,
    pub log_buffer: LogBuffer,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_running: false,
            miner_running: false,
            explorer_server_running: false,
            headless_running: false,
            tx_mining_running: false,
            node_child_id: None,
            miner_child_id: None,
            headless_child_id: None,
            tx_mining_child_id: None,
            explorer_shutdown: None,
            data_dir: None,
            log_buffer: LogBuffer::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

/// Spawn a background task that reads lines from an async reader and pushes them
/// into the shared log buffer with a `[prefix]` tag.
fn spawn_log_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    log_buf: LogBuffer,
    prefix: &'static str,
) {
    tokio::spawn(async move {
        let br = BufReader::new(reader);
        let mut lines = br.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log_buf.push(format!("[{}] {}", prefix, line));
        }
    });
}

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
            stratum_port: 8003, // tx-mining-service stratum (handles both blocks and txs)
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
pub struct TxMiningConfig {
    pub api_port: u16,
    pub stratum_port: u16,
    pub fullnode_url: String,
    pub address: String,
}

impl Default for TxMiningConfig {
    fn default() -> Self {
        Self {
            api_port: 8002,
            stratum_port: 8003,
            fullnode_url: "http://localhost:8080".to_string(),
            address: "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxMiningStatus {
    pub running: bool,
    pub port: Option<u16>,
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

    // Windows binaries have .exe extension
    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    // Allow override via HATHOR_FORGE_BINARIES_DIR for Nix / custom installs
    let binaries_dir = match std::env::var("HATHOR_FORGE_BINARIES_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
    };

    // hathor-core and tx-mining-service use onedir mode (folder with binary inside)
    if name == "hathor-core" || name == "tx-mining-service" {
        let binary_name = format!("{}{}", name, exe_suffix);
        let onedir_path = binaries_dir
            .join(format!("{}-{}", name, target))
            .join(&binary_name);
        if onedir_path.exists() {
            return onedir_path;
        }
    }

    // For single-file binaries (cpuminer) with target triple suffix
    let dev_path = binaries_dir.join(format!("{}-{}{}", name, target, exe_suffix));
    if dev_path.exists() {
        return dev_path;
    }

    // Try bare name (for Nix or PATH-style layouts)
    let bare_path = binaries_dir.join(format!("{}{}", name, exe_suffix));
    if bare_path.exists() {
        return bare_path;
    }

    // Fallback to current dir
    std::path::PathBuf::from("binaries").join(format!("{}-{}{}", name, target, exe_suffix))
}

// Set platform-specific library path environment variable for bundled libraries
fn set_library_path_env(cmd: &mut TokioCommand, internal_dir: &std::path::Path) {
    #[cfg(target_os = "macos")]
    cmd.env("DYLD_FALLBACK_LIBRARY_PATH", internal_dir);

    #[cfg(target_os = "linux")]
    cmd.env("LD_LIBRARY_PATH", internal_dir);

    // Windows uses PATH or same directory as executable, no special env needed
    #[cfg(target_os = "windows")]
    let _ = (cmd, internal_dir); // Suppress unused warnings
}

// Get the path to the wallet-headless-dist directory
fn get_headless_dist_path() -> std::path::PathBuf {
    // Allow override via HATHOR_FORGE_HEADLESS_DIR for Nix / custom installs
    if let Ok(dir) = std::env::var("HATHOR_FORGE_HEADLESS_DIR") {
        return std::path::PathBuf::from(dir);
    }

    // In dev mode, wallet-headless-dist is in src-tauri/wallet-headless-dist/
    let dev_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wallet-headless-dist");
    if dev_path.exists() {
        return dev_path;
    }

    // Fallback to current dir
    std::path::PathBuf::from("wallet-headless-dist")
}

// Generate wallet-headless config file in the dist directory
fn generate_headless_config(
    config: &HeadlessConfig,
    headless_dist_path: &std::path::Path,
) -> Result<(), String> {
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
  txMiningUrl: 'http://localhost:8002',
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
                    let _ = Command::new("kill")
                        .args(["-9", &pid_num.to_string()])
                        .output();
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        // On Windows, use netstat to find the PID and taskkill to kill it
        if let Ok(output) = Command::new("netstat").args(["-ano", "-p", "TCP"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
                    if let Some(pid) = line.split_whitespace().last() {
                        let _ = Command::new("taskkill").args(["/PID", pid, "/F"]).output();
                    }
                }
            }
        }
    }
}

// ============================================================================
// Internal Functions (used by both Tauri commands and MCP server)
// ============================================================================

/// Start the Hathor fullnode (internal version without Tauri AppHandle)
pub async fn start_node_internal(state: &SharedState) -> Result<String, String> {
    let config = NodeConfig::default();
    let state_guard = state.lock().await;

    if state_guard.node_running {
        return Ok("Node is already running".to_string());
    }

    // Kill any zombie processes from previous runs
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    kill_process_on_port(8001); // wallet-headless port
    kill_process_on_port(8002); // tx-mining-service port
    kill_process_on_port(8003); // tx-mining-service stratum port

    drop(state_guard);

    // Give the OS a moment to release the ports
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut state_guard = state.lock().await;

    let binary_path = get_binary_path("hathor-core");

    // Ensure data directory exists
    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    // Development HD wallet seed (DO NOT use in production!)
    let dev_wallet_words = "avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level";

    // Set platform-specific library path for bundled libraries
    let internal_dir = binary_path.parent().unwrap().join("_internal");

    // Spawn the process using tokio
    let mut cmd = TokioCommand::new(&binary_path);
    set_library_path_env(&mut cmd, &internal_dir);
    let mut child = cmd
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
            "--nc-exec-logs",
            "all",
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

    // Route stdout/stderr to shared log buffer
    let log_buf = state_guard.log_buffer.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let state_clone = state.clone();

    if let Some(out) = stdout {
        spawn_log_reader(out, log_buf.clone(), "node");
    }
    if let Some(err) = stderr {
        spawn_log_reader(err, log_buf, "node");
    }

    // Spawn task to wait for process termination and reset state
    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut state_guard = state_clone.lock().await;
        state_guard.node_running = false;
        state_guard.node_child_id = None;
    });

    Ok(format!("Node started on port {}", config.api_port))
}

/// Stop the Hathor fullnode (internal version)
pub async fn stop_node_internal(state: &SharedState) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    // First stop miner if running
    if let Some(pid) = state_guard.miner_child_id {
        kill_process(pid);
        state_guard.miner_running = false;
        state_guard.miner_child_id = None;
    }

    // Stop headless if running
    if let Some(pid) = state_guard.headless_child_id {
        kill_process(pid);
        state_guard.headless_running = false;
        state_guard.headless_child_id = None;
    }

    // Stop tx-mining-service if running
    if let Some(pid) = state_guard.tx_mining_child_id {
        kill_process(pid);
        state_guard.tx_mining_running = false;
        state_guard.tx_mining_child_id = None;
    }

    // Stop node
    if !state_guard.node_running {
        return Ok("Node is not running".to_string());
    }

    if let Some(pid) = state_guard.node_child_id {
        kill_process(pid);
    }

    state_guard.node_running = false;
    state_guard.node_child_id = None;

    Ok("Node stopped".to_string())
}

/// Start the CPU miner (internal version)
pub async fn start_miner_internal(
    state: &SharedState,
    address: Option<String>,
) -> Result<String, String> {
    let config = MinerConfig {
        address: address.unwrap_or_else(|| "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb".to_string()),
        ..MinerConfig::default()
    };

    {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Err("Node must be running before starting miner".to_string());
        }
        if state_guard.miner_running {
            return Ok("Miner is already running".to_string());
        }
        // Auto-start tx-mining-service if not running (miner connects to its stratum)
        if !state_guard.tx_mining_running {
            drop(state_guard);
            start_tx_mining_internal(state).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    let mut state_guard = state.lock().await;

    let binary_path = get_binary_path("cpuminer");

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

    let log_buf = state_guard.log_buffer.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let state_clone = state.clone();

    if let Some(out) = stdout {
        spawn_log_reader(out, log_buf.clone(), "miner");
    }
    if let Some(err) = stderr {
        spawn_log_reader(err, log_buf, "miner");
    }

    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut state_guard = state_clone.lock().await;
        state_guard.miner_running = false;
        state_guard.miner_child_id = None;
    });

    Ok(format!("Miner started with {} threads", config.threads))
}

/// Stop the CPU miner (internal version)
pub async fn stop_miner_internal(state: &SharedState) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.miner_running {
        return Ok("Miner is not running".to_string());
    }

    if let Some(pid) = state_guard.miner_child_id {
        kill_process(pid);
    }

    state_guard.miner_running = false;
    state_guard.miner_child_id = None;

    Ok("Miner stopped".to_string())
}

/// Start the wallet-headless service (internal version)
pub async fn start_headless_internal(state: &SharedState) -> Result<String, String> {
    let config = HeadlessConfig::default();
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting wallet-headless".to_string());
    }

    if state_guard.headless_running {
        return Ok("Wallet-headless is already running".to_string());
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

    drop(state_guard);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    let mut state_guard = state.lock().await;

    // Generate config file
    generate_headless_config(&config, &headless_path)?;

    let entry_point = headless_path.join("dist").join("index.js");
    let working_dir = headless_path.join("dist");

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

    let log_buf = state_guard.log_buffer.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let state_clone = state.clone();

    if let Some(out) = stdout {
        spawn_log_reader(out, log_buf.clone(), "wallet");
    }
    if let Some(err) = stderr {
        spawn_log_reader(err, log_buf, "wallet");
    }

    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut state_guard = state_clone.lock().await;
        state_guard.headless_running = false;
        state_guard.headless_child_id = None;
    });

    Ok(format!("Wallet-headless started on port {}", config.port))
}

/// Stop the wallet-headless service (internal version)
pub async fn stop_headless_internal(state: &SharedState) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Ok("Wallet-headless is not running".to_string());
    }

    if let Some(pid) = state_guard.headless_child_id {
        kill_process(pid);
    }

    state_guard.headless_running = false;
    state_guard.headless_child_id = None;

    Ok("Wallet-headless stopped".to_string())
}

/// Start the tx-mining-service (internal version)
pub async fn start_tx_mining_internal(state: &SharedState) -> Result<String, String> {
    let config = TxMiningConfig::default();
    let state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting tx-mining-service".to_string());
    }

    if state_guard.tx_mining_running {
        return Ok("tx-mining-service is already running".to_string());
    }

    drop(state_guard);

    // Kill any zombie process on the tx-mining ports
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let mut state_guard = state.lock().await;

    let binary_path = get_binary_path("tx-mining-service");

    // Set platform-specific library path for bundled libraries
    let internal_dir = binary_path.parent().unwrap().join("_internal");

    let mut cmd = TokioCommand::new(&binary_path);
    set_library_path_env(&mut cmd, &internal_dir);
    let mut child = cmd
        .args([
            "--api-port",
            &config.api_port.to_string(),
            "--stratum-port",
            &config.stratum_port.to_string(),
            "--address",
            &config.address,
            "--allow-non-standard-script",
            "--tx-timeout",
            "120",
            &config.fullnode_url,
        ])
        .current_dir(binary_path.parent().unwrap())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to spawn tx-mining-service at {:?}: {}",
                binary_path, e
            )
        })?;

    let pid = child.id().unwrap_or(0);
    state_guard.tx_mining_running = true;
    state_guard.tx_mining_child_id = Some(pid);

    let log_buf = state_guard.log_buffer.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let state_clone = state.clone();

    if let Some(out) = stdout {
        spawn_log_reader(out, log_buf.clone(), "tx-mining");
    }
    if let Some(err) = stderr {
        spawn_log_reader(err, log_buf, "tx-mining");
    }

    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut state_guard = state_clone.lock().await;
        state_guard.tx_mining_running = false;
        state_guard.tx_mining_child_id = None;
    });

    Ok(format!(
        "tx-mining-service started on port {}",
        config.api_port
    ))
}

/// Stop the tx-mining-service (internal version)
pub async fn stop_tx_mining_internal(state: &SharedState) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.tx_mining_running {
        return Ok("tx-mining-service is not running".to_string());
    }

    if let Some(pid) = state_guard.tx_mining_child_id {
        kill_process(pid);
    }

    state_guard.tx_mining_running = false;
    state_guard.tx_mining_child_id = None;

    Ok("tx-mining-service stopped".to_string())
}

/// Generate a new BIP39 seed phrase (internal version)
pub fn generate_seed_internal() -> Result<String, String> {
    use bip39::{Language, Mnemonic};

    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| format!("Failed to generate random bytes: {}", e))?;

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
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
        // Poll for up to 5 seconds before force-killing
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Check if process is still alive (kill -0 returns error if dead)
            if let Ok(output) = Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output()
            {
                if !output.status.success() {
                    return; // Process already exited
                }
            } else {
                return; // Process already exited
            }
        }
        // Force kill if still running after 5 seconds
        eprintln!("Process {} did not exit after SIGTERM, sending SIGKILL", pid);
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

// MCP Server port
pub const MCP_SERVER_PORT: u16 = 9876;

// ============================================================================
// Tauri GUI app (only compiled with tauri-app feature)
// ============================================================================

#[cfg(feature = "tauri-app")]
mod tauri_app;

#[cfg(feature = "tauri-app")]
pub use tauri_app::run;
