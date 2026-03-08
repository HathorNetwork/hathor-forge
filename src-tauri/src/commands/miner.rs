use crate::*;
use std::process::Stdio;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

// Start the CPU miner
#[tauri::command]
pub(crate) async fn start_miner(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<MinerConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();

    {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Err("Node must be running before starting miner".to_string());
        }
        if state_guard.miner_running {
            return Err("Miner is already running".to_string());
        }
        // Auto-start tx-mining-service if not running (miner connects to its stratum)
        if !state_guard.tx_mining_running {
            drop(state_guard);
            start_tx_mining_internal(&state).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    let mut state_guard = state.lock().await;
    let binary_path = get_binary_path("cpuminer");

    // Spawn the process using tokio
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

    let pid = child.id();
    if pid.is_none() {
        eprintln!("Miner process exited immediately; no PID available");
    }
    state_guard.miner_running = true;
    state_guard.miner_child_id = pid;

    // Handle stdout
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_handle = app.clone();
    let app_handle2 = app.clone();

    // Spawn task for stdout
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit("miner-log", &line);
            }
        });
    }

    // Spawn task for stderr (cpuminer outputs stats here)
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle2.emit("miner-stats", &line);
            }
        });
    }

    // Spawn task to wait for process termination and reset state
    let app_handle3 = app.clone();
    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.map(|s| s.code()).ok().flatten();

        // Reset state when process terminates
        {
            let mut state_guard = state_clone.lock().await;
            state_guard.miner_running = false;
            state_guard.miner_child_id = None;
        }

        let _ = app_handle3.emit("miner-terminated", code);
    });

    Ok(format!("Miner started with {} threads", config.threads))
}

// Stop the CPU miner
#[tauri::command]
pub(crate) async fn stop_miner(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.miner_running {
        return Err("Miner is not running".to_string());
    }

    // Kill the process
    if let Some(pid) = state_guard.miner_child_id {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    state_guard.miner_running = false;
    state_guard.miner_child_id = None;

    Ok("Miner stopped".to_string())
}

// Start the tx-mining-service
#[tauri::command]
pub(crate) async fn start_tx_mining(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let config = TxMiningConfig::default();
    let mut state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting tx-mining-service".to_string());
    }

    if state_guard.tx_mining_running {
        return Err("tx-mining-service is already running".to_string());
    }

    // Kill any zombie process on the tx-mining ports
    kill_process_on_port(config.api_port);
    kill_process_on_port(config.stratum_port);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

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

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_handle = app.clone();
    let app_handle2 = app.clone();

    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit("tx-mining-log", &line);
            }
        });
    }

    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle2.emit("tx-mining-log", &line);
            }
        });
    }

    let app_handle3 = app.clone();
    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.map(|s| s.code()).ok().flatten();

        {
            let mut state_guard = state_clone.lock().await;
            state_guard.tx_mining_running = false;
            state_guard.tx_mining_child_id = None;
        }

        let _ = app_handle3.emit("tx-mining-terminated", code);
    });

    Ok(format!(
        "tx-mining-service started on port {}",
        config.api_port
    ))
}

// Stop the tx-mining-service
#[tauri::command]
pub(crate) async fn stop_tx_mining(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.tx_mining_running {
        return Err("tx-mining-service is not running".to_string());
    }

    if let Some(pid) = state_guard.tx_mining_child_id {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    state_guard.tx_mining_running = false;
    state_guard.tx_mining_child_id = None;

    Ok("tx-mining-service stopped".to_string())
}

// Get tx-mining-service status
#[tauri::command]
pub(crate) async fn get_tx_mining_status(
    state: tauri::State<'_, SharedState>,
) -> Result<TxMiningStatus, String> {
    let state_guard = state.lock().await;

    Ok(TxMiningStatus {
        running: state_guard.tx_mining_running,
        port: if state_guard.tx_mining_running {
            Some(8002)
        } else {
            None
        },
    })
}
