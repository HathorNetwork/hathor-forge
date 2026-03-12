use crate::services::headless::{start_headless_internal, stop_headless_internal};
use crate::state::SharedState;
use crate::types::{
    CreateHeadlessWalletRequest, HeadlessStatus, HeadlessWallet, HeadlessWalletBalance,
    HeadlessWalletSendTxRequest,
};

// Start the wallet-headless service
#[tauri::command]
pub(crate) async fn start_headless(
    state: tauri::State<'_, SharedState>,
    config: Option<crate::config::HeadlessConfig>,
) -> Result<String, String> {
    let fullnode_url = config.as_ref().map(|c| c.fullnode_url.as_str());
    start_headless_internal(&state, fullnode_url, None).await
}

// Stop the wallet-headless service
#[tauri::command]
pub(crate) async fn stop_headless(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    stop_headless_internal(&state).await
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
            Some(state_guard.ports.wallet_headless)
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://localhost:{}/start", headless_port))
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://localhost:{}/wallet/status", headless_port))
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://localhost:{}/wallet/balance", headless_port))
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "http://localhost:{}/wallet/addresses",
            headless_port
        ))
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .post(format!(
            "http://localhost:{}/wallet/simple-send-tx",
            headless_port
        ))
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
    let headless_port = {
        let state_guard = state.lock().await;
        if !state_guard.headless_running {
            return Err("Wallet-headless is not running".to_string());
        }
        state_guard.ports.wallet_headless
    };

    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://localhost:{}/wallet/stop", headless_port))
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
