use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

// Application state
#[derive(Default)]
pub struct AppState {
    node_running: bool,
    miner_running: bool,
    node_child_id: Option<u32>,
    miner_child_id: Option<u32>,
    data_dir: Option<String>,
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
        "data_dir": state_guard.data_dir,
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(Mutex::new(AppState::default())) as SharedState)
        .invoke_handler(tauri::generate_handler![
            start_node,
            stop_node,
            start_miner,
            stop_miner,
            get_node_status,
            get_miner_status,
            get_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
