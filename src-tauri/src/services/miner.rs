use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::MinerConfig;
use crate::platform::get_binary_path;
use crate::process::{setup_child_logging, spawn_exit_monitor, stop_service};
use crate::state::SharedState;

use super::tx_mining::start_tx_mining_internal;

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
        if !state_guard.tx_mining_running {
            drop(state_guard);
            start_tx_mining_internal(state).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    let mut state_guard = state.lock().await;

    // Re-check after re-acquiring the lock to prevent TOCTOU race condition:
    // another caller may have started the miner while we released the lock.
    if state_guard.miner_running {
        return Ok("Miner is already running".to_string());
    }

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

    let pid = setup_child_logging(&mut child, state_guard.log_buffer.clone(), "miner");
    state_guard.miner_running = true;
    state_guard.miner_child_id = pid;

    spawn_exit_monitor(child, state.clone(), |s| {
        s.miner_running = false;
        s.miner_child_id = None;
    });

    Ok(format!("Miner started with {} threads", config.threads))
}

/// Stop the CPU miner (internal version)
pub async fn stop_miner_internal(state: &SharedState) -> Result<String, String> {
    stop_service(
        state,
        "Miner",
        |s| s.miner_running,
        |s| {
            s.miner_running = false;
            s.miner_child_id.take()
        },
    )
    .await
}
