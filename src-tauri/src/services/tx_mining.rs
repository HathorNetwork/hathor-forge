use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::TxMiningConfig;
use crate::platform::{get_binary_path, kill_process, kill_process_on_port, set_library_path_env};
use crate::state::{spawn_log_reader, SharedState};

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

    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let mut state_guard = state.lock().await;

    // Re-check after re-acquiring the lock to prevent TOCTOU race condition:
    // another caller may have started tx-mining-service while we released the lock.
    if state_guard.tx_mining_running {
        return Ok("tx-mining-service is already running".to_string());
    }

    let binary_path = get_binary_path("tx-mining-service");

    let binary_parent = binary_path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory of {:?}", binary_path))?;
    let internal_dir = binary_parent.join("_internal");

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
        .current_dir(binary_parent)
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

    let pid = child.id();
    if pid.is_none() {
        eprintln!("tx-mining-service process exited immediately; no PID available");
    }
    state_guard.tx_mining_running = true;
    state_guard.tx_mining_child_id = pid;

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
        kill_process(pid).await;
    }

    state_guard.tx_mining_running = false;
    state_guard.tx_mining_child_id = None;

    Ok("tx-mining-service stopped".to_string())
}
