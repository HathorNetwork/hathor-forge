use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::MinerConfig;
use crate::platform::{get_binary_path, kill_process};
use crate::state::{spawn_log_reader, SharedState};

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
