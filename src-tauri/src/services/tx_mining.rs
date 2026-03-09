use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::TxMiningConfig;
use crate::platform::{get_binary_path, kill_process_on_port, set_library_path_env};
use crate::process::{setup_child_logging, spawn_exit_monitor, stop_service};
use crate::state::SharedState;

/// Start the tx-mining-service (internal version)
pub async fn start_tx_mining_internal(state: &SharedState) -> Result<String, String> {
    let ports = state.lock().await.ports.clone();
    let config = TxMiningConfig {
        api_port: ports.tx_mining_api,
        stratum_port: ports.tx_mining_stratum,
        fullnode_url: format!("http://localhost:{}", ports.fullnode_api),
        ..TxMiningConfig::default()
    };
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

    // Re-check ALL preconditions after re-acquiring the lock to prevent TOCTOU race:
    // the node may have stopped, or another caller may have started tx-mining-service.
    if !state_guard.node_running {
        return Err("Node must be running before starting tx-mining-service".to_string());
    }
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

    let pid = setup_child_logging(&mut child, state_guard.log_buffer.clone(), "tx-mining", state_guard.app_handle.clone());
    state_guard.tx_mining_running = true;
    state_guard.tx_mining_child_id = pid;

    spawn_exit_monitor(child, state.clone(), |s| {
        s.tx_mining_running = false;
        s.tx_mining_child_id = None;
    }, |s| s.tx_mining_child_id, "tx-mining-terminated");

    Ok(format!(
        "tx-mining-service started on port {}",
        config.api_port
    ))
}

/// Stop the tx-mining-service (internal version)
pub async fn stop_tx_mining_internal(state: &SharedState) -> Result<String, String> {
    stop_service(
        state,
        "tx-mining-service",
        |s| s.tx_mining_running,
        |s| {
            s.tx_mining_running = false;
            s.tx_mining_child_id.take()
        },
    )
    .await
}
