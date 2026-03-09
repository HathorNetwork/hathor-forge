use crate::*;
use std::fs;
use std::process::Stdio;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

// Start the Hathor fullnode
#[tauri::command]
pub(crate) async fn start_node(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<NodeConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    // Check state and bail early if already running (without holding lock during cleanup)
    {
        let state_guard = state.lock().await;
        if state_guard.node_running {
            return Err("Node is already running".to_string());
        }
    }

    // Kill any zombie processes from previous runs (no lock held)
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    kill_process_on_port(crate::config::DEFAULT_WALLET_HEADLESS_PORT);
    kill_process_on_port(crate::config::DEFAULT_TX_MINING_API_PORT);
    kill_process_on_port(crate::config::DEFAULT_TX_MINING_STRATUM_PORT);
    // Give the OS a moment to release the ports
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Re-acquire lock and re-check all preconditions before proceeding
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

    // Set platform-specific library path for bundled libraries
    let internal_dir = binary_path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory of {:?}", binary_path))?
        .join("_internal");

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

    let pid = child.id();
    if pid.is_none() {
        eprintln!("Node process exited immediately; no PID available");
    }
    state_guard.node_running = true;
    state_guard.node_child_id = pid;
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
pub(crate) async fn stop_node(state: tauri::State<'_, SharedState>) -> Result<String, String> {
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

// Get node status from the API
#[tauri::command]
pub(crate) async fn get_node_status(
    state: tauri::State<'_, SharedState>,
) -> Result<NodeStatus, String> {
    {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Ok(NodeStatus {
                running: false,
                block_height: None,
                hash_rate: None,
                peer_count: None,
            });
        }
    }
    // Lock released before making HTTP request to avoid holding it across network I/O

    // Try to fetch status from the node API
    let client = reqwest::Client::new();
    match client
        .get(format!(
            "http://127.0.0.1:{}/v1a/status",
            crate::config::DEFAULT_FULLNODE_API_PORT
        ))
        .send()
        .await
    {
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
pub(crate) async fn get_miner_status(
    state: tauri::State<'_, SharedState>,
) -> Result<MinerStatus, String> {
    let state_guard = state.lock().await;

    Ok(MinerStatus {
        running: state_guard.miner_running,
        hash_rate: None, // TODO: Parse from miner output
    })
}

// Get current state
#[tauri::command]
pub(crate) async fn get_state(
    state: tauri::State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;

    Ok(serde_json::json!({
        "node_running": state_guard.node_running,
        "miner_running": state_guard.miner_running,
        "explorer_server_running": state_guard.explorer_server_running,
        "headless_running": state_guard.headless_running,
        "tx_mining_running": state_guard.tx_mining_running,
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
pub(crate) async fn reset_data(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    // Hold lock through the entire operation to prevent the node from starting
    // between the check and the directory removal
    let state_guard = state.lock().await;

    if state_guard.node_running {
        return Err("Cannot reset data while node is running. Stop the node first.".to_string());
    }

    let data_dir = state_guard
        .data_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_default_data_dir);

    // fs::remove_dir_all is synchronous but fast enough to hold across.
    // Holding the lock prevents a concurrent start_node from racing.
    if data_dir.exists() {
        fs::remove_dir_all(&data_dir)
            .map_err(|e| format!("Failed to remove data directory: {}", e))?;
    }

    Ok(format!("Data directory removed: {:?}", data_dir))
}

/// Returns the Claude Desktop MCP config snippet with resolved paths to the
/// bundled Node.js binary and the stdio bridge script.
#[cfg(feature = "tauri-app")]
#[tauri::command]
pub(crate) async fn get_mcp_config() -> Result<serde_json::Value, String> {
    let node_path = crate::platform::get_node_binary_path()?;

    // Try dev path first, then production path next to the executable
    let bridge_name = "mcp-stdio-bridge.mjs";
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(bridge_name);
    let bridge_path = if dev_path.exists() {
        dev_path
    } else if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe
            .parent()
            .ok_or("Cannot determine parent directory of executable")?;

        // macOS: Tauri places resources in Contents/Resources/
        #[cfg(target_os = "macos")]
        let resources_path = {
            let p = exe_dir.join("../Resources").join(bridge_name);
            if p.exists() {
                Some(p)
            } else {
                None
            }
        };
        #[cfg(not(target_os = "macos"))]
        let resources_path: Option<std::path::PathBuf> = None;

        if let Some(p) = resources_path {
            p
        } else {
            let prod_path = exe_dir.join(bridge_name);
            if prod_path.exists() {
                prod_path
            } else {
                // Windows: check resources/ subdirectory
                #[cfg(target_os = "windows")]
                {
                    let win_path = exe_dir.join("resources").join(bridge_name);
                    if win_path.exists() {
                        return Ok(serde_json::json!({
                            "hathor-forge": {
                                "command": node_path.to_string_lossy(),
                                "args": [
                                    win_path.to_string_lossy(),
                                    &format!("http://127.0.0.1:{}/mcp", crate::config::DEFAULT_MCP_SERVER_PORT)
                                ]
                            }
                        }));
                    }
                }
                return Err(format!(
                    "MCP bridge script not found at {:?} or {:?}",
                    dev_path, prod_path
                ));
            }
        }
    } else {
        return Err(format!("MCP bridge script not found at {:?}", dev_path));
    };

    Ok(serde_json::json!({
        "hathor-forge": {
            "command": node_path.to_string_lossy(),
            "args": [
                bridge_path.to_string_lossy(),
                &format!("http://127.0.0.1:{}/mcp", crate::config::DEFAULT_MCP_SERVER_PORT)
            ]
        }
    }))
}
