use std::fs;
use std::process::Stdio;
use tauri::Emitter;
use tokio::process::Command as TokioCommand;

use crate::config::NodeConfig;
use crate::platform::{get_binary_path, kill_process, kill_process_on_port, set_library_path_env};
use crate::process::{setup_child_logging, spawn_exit_monitor};
use crate::state::SharedState;

/// Start the Hathor fullnode (internal version without Tauri AppHandle)
pub async fn start_node_internal(state: &SharedState) -> Result<String, String> {
    let state_guard = state.lock().await;
    let config = NodeConfig::from_ports(&state_guard.ports);
    let ports = state_guard.ports.clone();

    if state_guard.node_running {
        return Ok("Node is already running".to_string());
    }

    // Kill any zombie processes from previous runs
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    kill_process_on_port(ports.wallet_headless);
    kill_process_on_port(ports.tx_mining_api);
    kill_process_on_port(ports.tx_mining_stratum);

    drop(state_guard);

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut state_guard = state.lock().await;

    // Re-check after re-acquiring the lock to prevent TOCTOU race condition:
    // another caller may have started the node while we released the lock.
    if state_guard.node_running {
        return Ok("Node is already running".to_string());
    }

    let binary_path = get_binary_path("hathor-core");

    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    let dev_wallet_words = "avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level";

    let internal_dir = binary_path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory of {:?}", binary_path))?
        .join("_internal");

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

    let app_handle = state_guard.app_handle.clone();
    let pid = setup_child_logging(&mut child, state_guard.log_buffer.clone(), "node", app_handle.clone());
    state_guard.node_running = true;
    state_guard.node_child_id = pid;
    state_guard.data_dir = Some(config.data_dir.clone());

    if let Some(ref handle) = app_handle {
        let _ = handle.emit("node-started", ());
    }

    spawn_exit_monitor(child, state.clone(), |s| {
        s.node_running = false;
        s.node_child_id = None;
    }, |s| s.node_child_id, "node-terminated");

    Ok(format!("Node started on port {}", config.api_port))
}

/// Stop the Hathor fullnode (internal version)
pub async fn stop_node_internal(state: &SharedState) -> Result<String, String> {
    // Extract all PIDs and update state, then drop the lock before killing processes.
    let (miner_pid, headless_pid, tx_mining_pid, node_pid) = {
        let mut guard = state.lock().await;

        let miner_pid = guard.miner_child_id.take();
        if miner_pid.is_some() {
            guard.miner_running = false;
        }

        let headless_pid = guard.headless_child_id.take();
        if headless_pid.is_some() {
            guard.headless_running = false;
        }

        let tx_mining_pid = guard.tx_mining_child_id.take();
        if tx_mining_pid.is_some() {
            guard.tx_mining_running = false;
        }

        if !guard.node_running {
            return Ok("Node is not running".to_string());
        }

        let node_pid = guard.node_child_id.take();
        guard.node_running = false;

        (miner_pid, headless_pid, tx_mining_pid, node_pid)
    };

    // Kill all processes without holding the lock
    if let Some(pid) = miner_pid {
        kill_process(pid).await;
    }
    if let Some(pid) = headless_pid {
        kill_process(pid).await;
    }
    if let Some(pid) = tx_mining_pid {
        kill_process(pid).await;
    }
    if let Some(pid) = node_pid {
        kill_process(pid).await;
    }

    Ok("Node stopped".to_string())
}
