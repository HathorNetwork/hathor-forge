use crate::*;
use std::process::Stdio;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

// Start the wallet-headless service
#[tauri::command]
pub(crate) async fn start_headless(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    config: Option<HeadlessConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    let mut state_guard = state.lock().await;

    if !state_guard.node_running {
        return Err("Node must be running before starting wallet-headless".to_string());
    }

    if state_guard.headless_running {
        return Err("Wallet-headless is already running".to_string());
    }

    let headless_path = get_headless_dist_path();
    if !headless_path.exists() {
        return Err(format!(
            "Wallet-headless dist not found at {:?}. Run 'build-wallet-headless' first.",
            headless_path
        ));
    }

    // Kill any zombie process on the headless port
    kill_process_on_port(config.port);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Generate config file in the dist directory
    generate_headless_config(&config, &headless_path, "http://localhost:8002")?;

    // Find node binary to run with
    let node_bin = get_node_binary_path()?;
    let entry_point = headless_path.join("dist").join("index.js");
    let working_dir = headless_path.join("dist");

    // Spawn the process using bundled node (working dir must be dist/ where config.js is)
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
                let _ = app_handle.emit("headless-log", &line);
            }
        });
    }

    // Spawn task for stderr
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle2.emit("headless-log", &line);
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
            state_guard.headless_running = false;
            state_guard.headless_child_id = None;
        }

        let _ = app_handle3.emit("headless-terminated", code);
    });

    Ok(format!("Wallet-headless started on port {}", config.port))
}

// Stop the wallet-headless service
#[tauri::command]
pub(crate) async fn stop_headless(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    // Kill the process
    if let Some(pid) = state_guard.headless_child_id {
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

    state_guard.headless_running = false;
    state_guard.headless_child_id = None;

    Ok("Wallet-headless stopped".to_string())
}

// Get headless status
#[tauri::command]
pub(crate) async fn get_headless_status(
    state: tauri::State<'_, SharedState>,
) -> Result<HeadlessStatus, String> {
    let state_guard = state.lock().await;

    Ok(HeadlessStatus {
        running: state_guard.headless_running,
        port: if state_guard.headless_running {
            Some(8001)
        } else {
            None
        },
    })
}

// Create a new wallet via wallet-headless
#[tauri::command]
pub(crate) async fn create_headless_wallet(
    state: tauri::State<'_, SharedState>,
    request: CreateHeadlessWalletRequest,
) -> Result<HeadlessWallet, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    // Start a wallet with the provided seed
    let response = client
        .post("http://localhost:8001/start")
        .json(&serde_json::json!({
            "wallet-id": request.wallet_id,
            "seed": request.seed,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create wallet: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(HeadlessWallet {
            wallet_id: request.wallet_id,
            status: "starting".to_string(),
            status_code: None,
        })
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to create wallet: {}", message))
    }
}

// Get wallet status from headless
#[tauri::command]
pub(crate) async fn get_headless_wallet_status(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<HeadlessWallet, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/status")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet status: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let status_code = result["statusCode"].as_i64().map(|c| c as i32);
    let status_message = result["statusMessage"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    Ok(HeadlessWallet {
        wallet_id,
        status: status_message,
        status_code,
    })
}

// Get wallet balance from headless
#[tauri::command]
pub(crate) async fn get_headless_wallet_balance(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<HeadlessWalletBalance, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/balance")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet balance: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let available = result["available"].as_u64().unwrap_or(0);
    let locked = result["locked"].as_u64().unwrap_or(0);

    Ok(HeadlessWalletBalance { available, locked })
}

// Get wallet addresses from headless
#[tauri::command]
pub(crate) async fn get_headless_wallet_addresses(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<Vec<String>, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8001/wallet/addresses")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to get wallet addresses: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let addresses = result["addresses"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(addresses)
}

// Send transaction from headless wallet
#[tauri::command]
pub(crate) async fn headless_wallet_send_tx(
    state: tauri::State<'_, SharedState>,
    request: HeadlessWalletSendTxRequest,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/simple-send-tx")
        .header("X-Wallet-Id", &request.wallet_id)
        .json(&serde_json::json!({
            "address": request.address,
            "value": request.amount,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send transaction: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let result: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, response_text))?;

    if result["success"].as_bool().unwrap_or(false) {
        let tx_hash = result["hash"].as_str().unwrap_or("unknown").to_string();
        Ok(format!("Transaction sent! Hash: {}", tx_hash))
    } else {
        // Try multiple error message locations
        let message = result["message"]
            .as_str()
            .or_else(|| result["error"].as_str())
            .unwrap_or(&response_text)
            .to_string();
        Err(format!("Transaction failed: {}", message))
    }
}

// Close a headless wallet
#[tauri::command]
pub(crate) async fn close_headless_wallet(
    state: tauri::State<'_, SharedState>,
    wallet_id: String,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/stop")
        .header("X-Wallet-Id", &wallet_id)
        .send()
        .await
        .map_err(|e| format!("Failed to close wallet: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(format!("Wallet '{}' closed", wallet_id))
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to close wallet: {}", message))
    }
}
