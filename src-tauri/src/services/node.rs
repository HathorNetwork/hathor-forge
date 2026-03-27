use std::fs;
use std::process::Stdio;
use tauri::Emitter;
use tokio::process::Command as TokioCommand;

use crate::config::NodeConfig;
use crate::platform::{get_binary_path, hide_console_window, kill_process, set_library_path_env};
use crate::process::{setup_child_logging, spawn_exit_monitor};
use crate::state::SharedState;

/// Kill any process currently listening on the given TCP port.
/// This cleans up orphaned processes from a previous unclean shutdown.
async fn kill_port_occupant(port: u16) {
    #[cfg(unix)]
    {
        // lsof -ti tcp:<port> returns PIDs of processes using that port
        if let Ok(output) = tokio::process::Command::new("lsof")
            .args(["-ti", &format!("tcp:{}", port)])
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    tracing::warn!(port, pid, "Killing orphaned process on port");
                    kill_process(pid).await;
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        if let Ok(output) = tokio::process::Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let port_str = format!(":{}", port);
            for line in stdout.lines() {
                if line.contains(&port_str) && line.contains("LISTENING") {
                    if let Some(pid_str) = line.split_whitespace().last() {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            tracing::warn!(port, pid, "Killing orphaned process on port");
                            kill_process(pid).await;
                        }
                    }
                }
            }
        }
    }
}

/// Start the Hathor fullnode (internal version without Tauri AppHandle)
pub async fn start_node_internal(state: &SharedState) -> Result<String, String> {
    let state_guard = state.lock().await;
    let config = NodeConfig::from_ports(&state_guard.ports);

    if state_guard.node_running {
        return Ok("Node is already running".to_string());
    }

    drop(state_guard);

    // Kill any orphaned processes from a previous unclean shutdown
    let public_api_port = state.lock().await.ports.fullnode_api;
    kill_port_occupant(public_api_port).await;
    kill_port_occupant(config.api_port).await;
    kill_port_occupant(config.stratum_port).await;

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

    // Write DAA config tuned for local development:
    // - 10s block interval (vs 30s mainnet) for faster iteration
    // - 20-block lookback (vs 134 mainnet) so DAA adapts quickly
    let config_yaml_path = std::path::PathBuf::from(&config.data_dir).join("forge-settings.yml");
    fs::write(
        &config_yaml_path,
        "extends: localnet.yml\nAVG_TIME_BETWEEN_BLOCKS: 10\nBLOCK_DIFFICULTY_N_BLOCKS: 20\nMAX_DISTANCE_BETWEEN_BLOCKS: 31536000\nREWARD_SPEND_MIN_BLOCKS: 0\n",
    )
    .map_err(|e| format!("Failed to write DAA config: {}", e))?;

    let config_yaml_str = config_yaml_path.to_string_lossy().to_string();

    let mut cmd = TokioCommand::new(&binary_path);
    // Force Python to use unbuffered stdout/stderr so log lines reach the
    // Tauri event pipe immediately (PyInstaller binaries respect this).
    cmd.env("PYTHONUNBUFFERED", "1");
    set_library_path_env(&mut cmd, &internal_dir);
    hide_console_window(&mut cmd);
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
            "--nc-indexes",
            "--unsafe-mode",
            "privatenet",
            "--config-yaml",
            &config_yaml_str,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn hathor-core at {:?}: {}", binary_path, e))?;

    let app_handle = state_guard.app_handle.clone();
    let pid = setup_child_logging(
        &mut child,
        state_guard.log_buffer.clone(),
        "node",
        app_handle.clone(),
    );
    state_guard.node_running = true;
    state_guard.node_child_id = pid;
    state_guard.data_dir = Some(config.data_dir.clone());

    // Start the CORS reverse proxy: public port (49080) → internal port (49180)
    let public_port = state_guard.ports.fullnode_api;
    let shutdown_tx = super::cors_proxy::start(public_port, config.api_port);
    state_guard.cors_proxy_shutdown = Some(shutdown_tx);

    if let Some(ref handle) = app_handle {
        let _ = handle.emit("node-started", ());
    }

    spawn_exit_monitor(
        child,
        state.clone(),
        |s| {
            s.node_running = false;
            s.node_child_id = None;
        },
        |s| s.node_child_id,
        "node-terminated",
    );

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

        // Shut down the CORS proxy
        if let Some(tx) = guard.cors_proxy_shutdown.take() {
            let _ = tx.send(());
        }

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
