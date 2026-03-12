use crate::services::node::{start_node_internal, stop_node_internal};
use crate::state::SharedState;
use crate::types::{MinerStatus, NodeStatus};
use std::fs;

// Start the Hathor fullnode
#[tauri::command]
pub(crate) async fn start_node(
    state: tauri::State<'_, SharedState>,
    config: Option<crate::config::NodeConfig>,
) -> Result<String, String> {
    let _ = config; // Config is derived from resolved ports in the services layer
    start_node_internal(&state).await
}

// Stop the Hathor fullnode
#[tauri::command]
pub(crate) async fn stop_node(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    stop_node_internal(&state).await
}

// Get node status from the API
#[tauri::command]
pub(crate) async fn get_node_status(
    state: tauri::State<'_, SharedState>,
) -> Result<NodeStatus, String> {
    let fullnode_port = state.lock().await.ports.fullnode_api;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    match client
        .get(format!("http://127.0.0.1:{}/v1a/status", fullnode_port))
        .send()
        .await
    {
        Ok(response) => {
            {
                let mut state_guard = state.lock().await;
                state_guard.node_running = true;
            }

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
                    peer_count: Some(0),
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
        Err(_) => {
            {
                let mut state_guard = state.lock().await;
                state_guard.node_running = false;
            }
            Ok(NodeStatus {
                running: false,
                block_height: None,
                hash_rate: None,
                peer_count: None,
            })
        }
    }
}

// Get miner status by checking if a cpuminer process is running on the system.
#[tauri::command]
pub(crate) async fn get_miner_status(
    state: tauri::State<'_, SharedState>,
) -> Result<MinerStatus, String> {
    #[cfg(unix)]
    let miner_running = {
        std::process::Command::new("pgrep")
            .arg("-x")
            .arg("cpuminer")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    #[cfg(windows)]
    let miner_running = {
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq cpuminer.exe", "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("cpuminer.exe"))
            .unwrap_or(false)
    };

    {
        let mut state_guard = state.lock().await;
        state_guard.miner_running = miner_running;
    }

    Ok(MinerStatus {
        running: miner_running,
        hash_rate: None,
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
    let state_guard = state.lock().await;

    if state_guard.node_running {
        return Err("Cannot reset data while node is running. Stop the node first.".to_string());
    }

    let data_dir = state_guard
        .data_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_default_data_dir);

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
pub(crate) async fn get_mcp_config(
    state: tauri::State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let mcp_port = state.lock().await.ports.mcp_server;
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
                                    &format!("http://127.0.0.1:{}/mcp", mcp_port)
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
                &format!("http://127.0.0.1:{}/mcp", mcp_port)
            ]
        }
    }))
}

/// Returns the machine's local network IP address (the one reachable from other devices on LAN).
#[tauri::command]
pub(crate) fn get_local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Returns the ports actually allocated for this run of the app.
#[tauri::command]
pub(crate) async fn get_ports(
    state: tauri::State<'_, SharedState>,
) -> Result<crate::config::ResolvedPorts, String> {
    Ok(state.lock().await.ports.clone())
}
