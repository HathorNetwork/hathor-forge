use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::config::HeadlessConfig;
use crate::platform::{
    detect_network_from_url, generate_headless_config, get_headless_dist_path,
    get_node_binary_path, kill_process, kill_process_on_port,
};
use crate::state::{spawn_log_reader, SharedState};

/// Start the wallet-headless service (internal version).
///
/// `fullnode_url` — fullnode server URL (e.g. `http://127.0.0.1:8080/v1a/`).
/// `tx_mining_url` — tx-mining-service URL (e.g. `http://localhost:8002`).
/// Both fall back to HeadlessConfig / localhost defaults when `None`.
pub async fn start_headless_internal(
    state: &SharedState,
    fullnode_url: Option<&str>,
    tx_mining_url: Option<&str>,
) -> Result<String, String> {
    let mut config = HeadlessConfig::default();
    if let Some(url) = fullnode_url {
        config.fullnode_url = if url.ends_with("/v1a/") {
            url.to_string()
        } else if url.ends_with('/') {
            format!("{}v1a/", url)
        } else {
            format!("{}/v1a/", url)
        };
        config.network = detect_network_from_url(url).to_string();
    }
    let txm_url = tx_mining_url.unwrap_or("http://localhost:8002");

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

    kill_process_on_port(config.port);

    drop(state_guard);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    let mut state_guard = state.lock().await;

    // Re-check after re-acquiring the lock to prevent TOCTOU race condition:
    // another caller may have started wallet-headless while we released the lock.
    if state_guard.headless_running {
        return Ok("Wallet-headless is already running".to_string());
    }

    generate_headless_config(&config, &headless_path, txm_url)?;

    let node_bin = get_node_binary_path()?;
    let entry_point = headless_path.join("dist").join("index.js");
    let working_dir = headless_path.join("dist");

    let mut child = TokioCommand::new(&node_bin)
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
        kill_process(pid).await;
    }

    state_guard.headless_running = false;
    state_guard.headless_child_id = None;

    Ok("Wallet-headless stopped".to_string())
}
