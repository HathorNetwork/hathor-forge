use crate::*;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateNanoContractRequest {
    pub wallet_id: String,
    pub blueprint_id: String,
    pub args: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallNanoContractMethodRequest {
    pub wallet_id: String,
    pub nc_id: String,
    pub method: String,
    pub args: Vec<serde_json::Value>,
}

// Get nano contract state
#[tauri::command]
pub(crate) async fn get_nano_contract_state(
    state: tauri::State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;
    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }
    drop(state_guard);

    let client = reqwest::Client::new();
    let response = client
        .get(&format!(
            "http://127.0.0.1:8080/v1a/nano_contract/state?id={}",
            id
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to get contract state: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(json)
}

// Get nano contract history
#[tauri::command]
pub(crate) async fn get_nano_contract_history(
    state: tauri::State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;
    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }
    drop(state_guard);

    let client = reqwest::Client::new();
    let response = client
        .get(&format!(
            "http://127.0.0.1:8080/v1a/nano_contract/history?id={}",
            id
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to get contract history: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(json)
}

// List available blueprints by scanning transactions for version 6 (blueprint) txs
#[tauri::command]
pub(crate) async fn list_blueprints(
    state: tauri::State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;
    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }
    drop(state_guard);

    let client = reqwest::Client::new();

    // Fetch recent transactions from dashboard (version 6 = blueprint)
    let response = client
        .get("http://127.0.0.1:8080/v1a/dashboard_tx?tx=200&block=0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch transactions: {}", e))?;

    let dashboard: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse dashboard response: {}", e))?;

    let mut blueprints = Vec::new();

    if let Some(transactions) = dashboard.get("transactions").and_then(|t| t.as_array()) {
        for tx in transactions {
            if tx.get("version").and_then(|v| v.as_u64()) == Some(6) {
                let tx_id = tx
                    .get("tx_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let timestamp = tx.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);

                // Fetch full transaction to get blueprint source code and extract class name
                let mut name = "Unknown".to_string();
                if let Ok(tx_response) = client
                    .get(&format!(
                        "http://127.0.0.1:8080/v1a/transaction?id={}",
                        tx_id
                    ))
                    .send()
                    .await
                {
                    if let Ok(tx_json) = tx_response.json::<serde_json::Value>().await {
                        if let Some(code_content) = tx_json
                            .get("tx")
                            .and_then(|t| t.get("on_chain_blueprint_code"))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            // Decode base64 + zlib to extract class name
                            if let Ok(decoded) =
                                base64::engine::general_purpose::STANDARD.decode(code_content)
                            {
                                if let Ok(decompressed) =
                                    miniz_oxide::inflate::decompress_to_vec_zlib(&decoded)
                                {
                                    if let Ok(source) = String::from_utf8(decompressed) {
                                        // Extract class name from "class XYZ(Blueprint):"
                                        for line in source.lines() {
                                            let trimmed = line.trim();
                                            if trimmed.starts_with("class ")
                                                && trimmed.contains("Blueprint")
                                            {
                                                if let Some(class_name) = trimmed
                                                    .strip_prefix("class ")
                                                    .and_then(|s| s.split('(').next())
                                                {
                                                    name = class_name.trim().to_string();
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                blueprints.push(serde_json::json!({
                    "id": tx_id,
                    "name": name,
                    "timestamp": timestamp,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "blueprints": blueprints,
    }))
}

// Get blueprint information by fetching the transaction and parsing the source code
#[tauri::command]
pub(crate) async fn get_blueprint_information(
    state: tauri::State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;
    if !state_guard.node_running {
        return Err("Node is not running".to_string());
    }
    drop(state_guard);

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://127.0.0.1:8080/v1a/transaction?id={}", id))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch blueprint transaction: {}", e))?;

    let tx_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let code_content = tx_json
        .get("tx")
        .and_then(|t| t.get("on_chain_blueprint_code"))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("Blueprint source code not found in transaction")?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(code_content)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(&decoded)
        .map_err(|e| format!("Failed to decompress: {:?}", e))?;

    let source = String::from_utf8(decompressed)
        .map_err(|e| format!("Invalid UTF-8 in blueprint source: {}", e))?;

    // Parse class name and methods from source
    let mut class_name = "Unknown".to_string();
    let mut methods = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        // Extract class name
        if trimmed.starts_with("class ") && trimmed.contains("Blueprint") {
            if let Some(name) = trimmed
                .strip_prefix("class ")
                .and_then(|s| s.split('(').next())
            {
                class_name = name.trim().to_string();
            }
        }
        // Extract method signatures: "def method_name(self, ctx: Context, arg1: Type1, ...)"
        if trimmed.starts_with("def ") && trimmed.contains("(self") {
            if let Some(sig) = trimmed.strip_prefix("def ") {
                let method_name = sig.split('(').next().unwrap_or("").trim().to_string();
                // Skip private methods
                if method_name.starts_with('_') {
                    continue;
                }
                let mut args = Vec::new();
                // Extract args between parens
                if let Some(params_str) = sig.split('(').nth(1).and_then(|s| s.split(')').next()) {
                    for param in params_str.split(',') {
                        let param = param.trim();
                        // Skip self and ctx parameters
                        if param == "self" || param.starts_with("ctx") {
                            continue;
                        }
                        if param.is_empty() {
                            continue;
                        }
                        let parts: Vec<&str> = param.splitn(2, ':').collect();
                        let arg_name = parts[0].trim().to_string();
                        let arg_type = if parts.len() > 1 {
                            parts[1].trim().to_string()
                        } else {
                            "str".to_string()
                        };
                        args.push(serde_json::json!({
                            "name": arg_name,
                            "type": arg_type,
                        }));
                    }
                }
                methods.push(serde_json::json!({
                    "name": method_name,
                    "args": args,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "id": id,
        "source": source,
        "plan": {
            "name": class_name,
            "methods": methods,
        }
    }))
}

// Create a new nano contract via wallet-headless (OCB)
#[tauri::command]
pub(crate) async fn headless_wallet_create_nano_contract(
    state: tauri::State<'_, SharedState>,
    request: CreateNanoContractRequest,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/nano-contracts/create")
        .header("X-Wallet-Id", &request.wallet_id)
        .json(&serde_json::json!({
            "blueprint_id": request.blueprint_id,
            "args": request.args,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create nano contract: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result)
}

// Call a nano contract method via wallet-headless
#[tauri::command]
pub(crate) async fn headless_wallet_call_nano_contract_method(
    state: tauri::State<'_, SharedState>,
    request: CallNanoContractMethodRequest,
) -> Result<serde_json::Value, String> {
    let state_guard = state.lock().await;

    if !state_guard.headless_running {
        return Err("Wallet-headless is not running".to_string());
    }

    drop(state_guard);

    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8001/wallet/nano-contracts/execute")
        .header("X-Wallet-Id", &request.wallet_id)
        .json(&serde_json::json!({
            "nc_id": request.nc_id,
            "method": request.method,
            "args": request.args,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to call nano contract method: {}", e))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result)
}
