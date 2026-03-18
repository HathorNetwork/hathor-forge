use std::process::Stdio;
use tauri::Emitter;
use tokio::process::Command as TokioCommand;

use crate::config::HeadlessConfig;
use crate::platform::{
    detect_network_from_url, generate_headless_config, get_headless_dist_path,
    get_node_binary_path, hide_console_window,
};
use crate::process::{setup_child_logging, spawn_exit_monitor, stop_service};
use crate::state::SharedState;

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
    let ports = state.lock().await.ports.clone();
    let mut config = HeadlessConfig {
        port: ports.wallet_headless,
        fullnode_url: format!("http://localhost:{}/v1a/", ports.fullnode_api),
        ..HeadlessConfig::default()
    };
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
    let txm_url = tx_mining_url
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("http://localhost:{}", ports.tx_mining_api));

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

    drop(state_guard);
    let mut state_guard = state.lock().await;

    // Re-check ALL preconditions after re-acquiring the lock to prevent TOCTOU race:
    // the node may have stopped, or another caller may have started wallet-headless.
    if !state_guard.node_running {
        return Err("Node must be running before starting wallet-headless".to_string());
    }
    if state_guard.headless_running {
        return Ok("Wallet-headless is already running".to_string());
    }

    generate_headless_config(&config, &headless_path, &txm_url)?;

    let node_bin = get_node_binary_path()?;
    let entry_point = headless_path.join("dist").join("index.js");
    let working_dir = headless_path.join("dist");

    let mut cmd = TokioCommand::new(&node_bin);
    hide_console_window(&mut cmd);

    // Set DYLD_FALLBACK_LIBRARY_PATH / LD_LIBRARY_PATH so the bundled Node.js
    // binary can find its dynamic libraries (libuv, libssl, etc.)
    // In production on macOS, node binary is in Contents/MacOS/ but dylibs are
    // in Contents/Resources/binaries/node-dylibs/ (bundled as resources).
    // In dev, they're in src-tauri/binaries/node-dylibs/.
    {
        let mut dylibs_candidates = Vec::new();
        if let Some(bin_dir) = node_bin.parent() {
            // Next to the binary (Linux/Windows production, or if co-located)
            dylibs_candidates.push(bin_dir.join("node-dylibs"));
            // macOS production: Contents/MacOS/../Resources/binaries/node-dylibs
            #[cfg(target_os = "macos")]
            dylibs_candidates.push(bin_dir.join("../Resources/binaries/node-dylibs"));
        }
        // Dev: CARGO_MANIFEST_DIR/binaries/node-dylibs
        dylibs_candidates.push(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/node-dylibs"),
        );
        for dylibs_dir in &dylibs_candidates {
            if dylibs_dir.exists() {
                #[cfg(target_os = "macos")]
                cmd.env("DYLD_FALLBACK_LIBRARY_PATH", dylibs_dir);
                #[cfg(target_os = "linux")]
                cmd.env("LD_LIBRARY_PATH", dylibs_dir);
                break;
            }
        }
    }

    let mut child = cmd
        .args([entry_point.to_string_lossy().as_ref()])
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn wallet-headless: {}", e))?;

    let pid = setup_child_logging(
        &mut child,
        state_guard.log_buffer.clone(),
        "wallet",
        state_guard.app_handle.clone(),
    );
    state_guard.headless_running = true;
    state_guard.headless_child_id = pid;

    if let Some(ref handle) = state_guard.app_handle {
        let _ = handle.emit("headless-started", ());
    }

    spawn_exit_monitor(
        child,
        state.clone(),
        |s| {
            s.headless_running = false;
            s.headless_child_id = None;
        },
        |s| s.headless_child_id,
        "headless-terminated",
    );

    Ok(format!("Wallet-headless started on port {}", config.port))
}

/// Stop the wallet-headless service (internal version)
pub async fn stop_headless_internal(state: &SharedState) -> Result<String, String> {
    stop_service(
        state,
        "Wallet-headless",
        |s| s.headless_running,
        |s| {
            s.headless_running = false;
            s.headless_child_id.take()
        },
    )
    .await
}
