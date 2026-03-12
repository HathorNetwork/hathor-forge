use crate::state::SharedState;
use crate::types::{FullnodeBalance, SendTxRequest, WalletAddress};

// Get wallet addresses with balances
#[tauri::command]
pub(crate) async fn get_wallet_addresses(
    state: tauri::State<'_, SharedState>,
) -> Result<Vec<WalletAddress>, String> {
    let fullnode_port = {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Err("Node is not running".to_string());
        }
        state_guard.ports.fullnode_api
    };

    let client = reqwest::Client::new();

    let address_response = client
        .get(format!(
            "http://127.0.0.1:{}/v1a/wallet/address",
            fullnode_port
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch address: {}", e))?;

    let address_json: serde_json::Value = address_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse address response: {}", e))?;

    let current_address = address_json["address"]
        .as_str()
        .ok_or("Invalid address format")?
        .to_string();

    let balance_response = client
        .get(format!(
            "http://127.0.0.1:{}/v1a/wallet/balance",
            fullnode_port
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch balance: {}", e))?;

    let balance_json: serde_json::Value = balance_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse balance response: {}", e))?;

    let balance = balance_json["balance"]["available"].as_u64();

    let wallet_addresses = vec![WalletAddress {
        address: current_address,
        index: 0,
        balance,
    }];

    Ok(wallet_addresses)
}

// Get fullnode wallet balance
#[tauri::command]
pub(crate) async fn get_fullnode_balance(
    state: tauri::State<'_, SharedState>,
) -> Result<FullnodeBalance, String> {
    let fullnode_port = {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Err("Node is not running".to_string());
        }
        state_guard.ports.fullnode_api
    };

    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "http://127.0.0.1:{}/v1a/wallet/balance/",
            fullnode_port
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to get balance: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let result: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, response_text))?;

    if result["success"].as_bool().unwrap_or(false) {
        let balance = &result["balance"];
        Ok(FullnodeBalance {
            available: balance["available"].as_i64().unwrap_or(0),
            locked: balance["locked"].as_i64().unwrap_or(0),
        })
    } else {
        let message = result["message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Failed to get balance: {}", message))
    }
}

// Send HTR to an address (faucet)
#[tauri::command]
pub(crate) async fn send_tx(
    state: tauri::State<'_, SharedState>,
    request: SendTxRequest,
) -> Result<String, String> {
    let fullnode_port = {
        let state_guard = state.lock().await;
        if !state_guard.node_running {
            return Err("Node is not running".to_string());
        }
        state_guard.ports.fullnode_api
    };

    let client = reqwest::Client::new();

    let response = client
        .post(format!(
            "http://127.0.0.1:{}/v1a/wallet/send_tokens/",
            fullnode_port
        ))
        .json(&serde_json::json!({
            "data": {
                "inputs": [],
                "outputs": [{
                    "address": request.address,
                    "value": request.amount,
                }]
            }
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
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Transaction failed: {}", message))
    }
}

// Generate a new BIP39 seed phrase (24 words)
#[tauri::command]
pub(crate) async fn generate_seed() -> Result<String, String> {
    use bip39::{Language, Mnemonic};

    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| format!("Failed to generate random bytes: {}", e))?;

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}
