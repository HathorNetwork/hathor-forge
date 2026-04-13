use std::fs;
use std::process::Stdio;
use tauri::Emitter;
use tokio::process::Command as TokioCommand;

use crate::config::NodeConfig;
use crate::platform::{get_binary_path, hide_console_window, kill_process, set_library_path_env};
#[cfg(windows)]
use crate::platform::send_ctrl_c_to_children;
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

    // Write full localnet config inlined (cannot use "extends: localnet.yml" because
    // the file doesn't resolve in PyInstaller bundles on Windows).
    // Source: hathorlib/hathorlib/conf/localnet.yml + testnet.yml merged + forge overrides.
    let config_yaml_path = std::path::PathBuf::from(&config.data_dir).join("forge-settings.yml");
    fs::write(
        &config_yaml_path,
        "\
P2PKH_VERSION_BYTE: x49\n\
MULTISIG_VERSION_BYTE: x87\n\
NETWORK_NAME: privatenet\n\
BOOTSTRAP_DNS: []\n\
GENESIS_OUTPUT_SCRIPT: 76a91466665b27f7dbc4c8c089d2f686c170c74d66f0b588ac\n\
GENESIS_BLOCK_TIMESTAMP: 1643902665\n\
GENESIS_BLOCK_NONCE: 4784939\n\
GENESIS_BLOCK_HASH: 00000334a21fbb58b4db8d7ff282d018e03e2977abd3004cf378fb1d677c3967\n\
GENESIS_TX1_NONCE: 0\n\
GENESIS_TX1_HASH: 54165cef1fd4cf2240d702b8383c307c822c16ca407f78014bdefa189a7571c2\n\
GENESIS_TX2_NONCE: 0\n\
GENESIS_TX2_HASH: 039906854ce6309b3180945f2a23deb9edff369753f7082e19053f5ac11bfbae\n\
MIN_TX_WEIGHT_K: 0\n\
MIN_TX_WEIGHT_COEFFICIENT: 0\n\
MIN_TX_WEIGHT: 1\n\
REWARD_SPEND_MIN_BLOCKS: 0\n\
CHECKPOINTS: []\n\
ENABLE_NANO_CONTRACTS: 'enabled'\n\
NC_ON_CHAIN_BLUEPRINT_RESTRICTED: false\n\
AVG_TIME_BETWEEN_BLOCKS: 10\n\
BLOCK_DIFFICULTY_N_BLOCKS: 20\n\
MAX_DISTANCE_BETWEEN_BLOCKS: 31536000\n",
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

    // On Windows, send CTRL_C to the shared hidden console so hathor-core
    // (Python/Twisted) catches it as KeyboardInterrupt and flushes RocksDB
    // before we force-kill. Without this step, taskkill /F below corrupts
    // the storage — "The last time you executed your full node it wasn't
    // stopped correctly". This mirrors the X-close path in RunEvent::Exit.
    #[cfg(windows)]
    {
        if let Some(pid) = node_pid {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            send_ctrl_c_to_children();

            // Wait up to 5s for hathor-core to exit on its own.
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match tokio::process::Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .await
                {
                    Ok(out) => {
                        if !String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()) {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    // Kill all processes without holding the lock. On Windows any that
    // already exited gracefully above make these calls no-ops.
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
