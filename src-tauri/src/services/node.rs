use std::fs;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::NodeConfig;
use crate::platform::{get_binary_path, kill_process, kill_process_on_port, set_library_path_env};
use crate::state::{spawn_log_reader, SharedState};

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
    kill_process_on_port(8001);
    kill_process_on_port(8002);
    kill_process_on_port(8003);

    drop(state_guard);

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut state_guard = state.lock().await;

    let binary_path = get_binary_path("hathor-core");

    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    let dev_wallet_words = "avocado spot town typical traffic vault danger century property shallow divorce festival spend attack anchor afford rotate green audit adjust fade wagon depart level";

    let internal_dir = binary_path.parent().unwrap().join("_internal");

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

    if let Some(pid) = state_guard.miner_child_id {
        kill_process(pid).await;
        state_guard.miner_running = false;
        state_guard.miner_child_id = None;
    }

    if let Some(pid) = state_guard.headless_child_id {
        kill_process(pid).await;
        state_guard.headless_running = false;
        state_guard.headless_child_id = None;
    }

    if let Some(pid) = state_guard.tx_mining_child_id {
        kill_process(pid).await;
        state_guard.tx_mining_running = false;
        state_guard.tx_mining_child_id = None;
    }

    if !state_guard.node_running {
        return Ok("Node is not running".to_string());
    }

    if let Some(pid) = state_guard.node_child_id {
        kill_process(pid).await;
    }

    state_guard.node_running = false;
    state_guard.node_child_id = None;

    Ok("Node stopped".to_string())
}
