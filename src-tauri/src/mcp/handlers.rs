//! Tool execution handlers for MCP tools.

use serde_json::{json, Value};
use std::time::Duration;

use super::types::McpState;

/// Execute an MCP tool by name with the given parameters.
pub async fn execute_tool(state: &McpState, name: &str, params: &Value) -> Result<String, String> {
    let client = state.http_client.clone();
    let fullnode_url = state.fullnode_url.read().await.clone();
    let wallet_headless_url = state.wallet_headless_url.read().await.clone();
    let _tx_mining_url = state.tx_mining_url.read().await.clone();

    match name {
        // Node Management
        "start_node" => crate::start_node_internal(&state.app_state).await,

        "stop_node" => crate::stop_node_internal(&state.app_state).await,

        "get_node_status" => {
            let app_state = state.app_state.lock().await;
            if !app_state.node_running {
                return Ok(json!({"running": false}).to_string());
            }
            drop(app_state);

            match client
                .get(format!("{}/v1a/status/", fullnode_url))
                .send()
                .await
            {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    Ok(format!(r#"{{"running": true, "status": {}}}"#, text))
                }
                Err(e) => Ok(json!({"running": false, "error": e.to_string()}).to_string()),
            }
        }

        // Miner Management
        "start_miner" => {
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .map(String::from);
            crate::start_miner_internal(&state.app_state, address).await
        }

        "stop_miner" => crate::stop_miner_internal(&state.app_state).await,

        "get_miner_status" => {
            let app_state = state.app_state.lock().await;
            Ok(json!({"running": app_state.miner_running}).to_string())
        }

        // Tx Mining Service
        "start_tx_mining" => crate::start_tx_mining_internal(&state.app_state).await,

        "stop_tx_mining" => crate::stop_tx_mining_internal(&state.app_state).await,

        "get_tx_mining_status" => {
            let app_state = state.app_state.lock().await;
            Ok(json!({
                "running": app_state.tx_mining_running,
                "port": if app_state.tx_mining_running { Some(8002) } else { None }
            })
            .to_string())
        }

        // Wallet Service
        "start_wallet_service" => {
            let fn_url = state.fullnode_url.read().await;
            let txm_url = state.tx_mining_url.read().await;
            crate::start_headless_internal(&state.app_state, Some(&fn_url), Some(&txm_url)).await
        }

        "stop_wallet_service" => crate::stop_headless_internal(&state.app_state).await,

        "get_wallet_service_status" => {
            let app_state = state.app_state.lock().await;
            Ok(json!({
                "running": app_state.headless_running,
                "port": if app_state.headless_running { Some(8001) } else { None }
            })
            .to_string())
        }

        // Wallet Operations
        "generate_seed" => crate::generate_seed_internal(),

        "create_wallet" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let seed = params.get("seed").and_then(|v| v.as_str());

            let wallet_seed = match seed {
                Some(s) => s.to_string(),
                None => crate::generate_seed_internal()?,
            };

            // Store seed
            state
                .wallet_seeds
                .lock()
                .await
                .insert(wallet_id.to_string(), wallet_seed.clone());

            // Create wallet via API
            let resp = client
                .post(format!("{}/start", wallet_headless_url))
                .json(&json!({
                    "wallet-id": wallet_id,
                    "seed": wallet_seed,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to create wallet: {}", e))?;

            let result: Value = resp
                .json()
                .await
                .unwrap_or(json!({"error": "Failed to parse response"}));
            let success = result
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let message = if success {
                if seed.is_some() {
                    "Wallet created with provided seed".to_string()
                } else {
                    "Wallet created with generated seed (use get_wallet_seed to retrieve)"
                        .to_string()
                }
            } else {
                result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Failed to create wallet in wallet-headless")
                    .to_string()
            };
            Ok(json!({
                "success": success,
                "wallet_id": wallet_id,
                "seed_stored": true,
                "message": message,
                "details": if !success { Some(&result) } else { None }
            })
            .to_string())
        }

        "get_wallet_seed" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;

            let seeds = state.wallet_seeds.lock().await;
            match seeds.get(wallet_id) {
                Some(seed) => Ok(json!({"wallet_id": wallet_id, "seed": seed}).to_string()),
                None => Ok(json!({"error": "Seed not found. Only seeds from wallets created in this session are stored."}).to_string()),
            }
        }

        "get_wallet_status" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;

            let resp = client
                .get(format!("{}/wallet/status", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .send()
                .await
                .map_err(|e| format!("Failed to get wallet status: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_wallet_balance" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;

            let resp = client
                .get(format!("{}/wallet/balance", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .send()
                .await
                .map_err(|e| format!("Failed to get wallet balance: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_wallet_addresses" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;

            let resp = client
                .get(format!("{}/wallet/addresses", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .send()
                .await
                .map_err(|e| format!("Failed to get wallet addresses: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "send_from_wallet" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("address is required")?;
            let amount = params
                .get("amount")
                .and_then(|v| v.as_f64())
                .ok_or("amount is required")?;

            let resp = client
                .post(format!("{}/wallet/simple-send-tx", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .json(&json!({
                    "address": address,
                    "value": (amount * 100.0) as i64,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to send transaction: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "close_wallet" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;

            let resp = client
                .post(format!("{}/wallet/stop", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .send()
                .await
                .map_err(|e| format!("Failed to close wallet: {}", e))?;

            state.wallet_seeds.lock().await.remove(wallet_id);

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        // Faucet
        "get_faucet_balance" => {
            let resp = client
                .get(format!("{}/v1a/wallet/balance/", fullnode_url))
                .send()
                .await
                .map_err(|e| format!("Failed to get faucet balance: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "send_from_faucet" => {
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("address is required")?;
            let amount = params
                .get("amount")
                .and_then(|v| v.as_f64())
                .ok_or("amount is required")?;

            let resp = client
                .post(format!("{}/v1a/wallet/send_tokens/", fullnode_url))
                .json(&json!({
                    "data": {
                        "inputs": [],
                        "outputs": [{
                            "address": address,
                            "value": (amount * 100.0) as i64,
                        }]
                    }
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to send from faucet: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "fund_wallet" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let amount = params.get("amount").and_then(|v| v.as_f64());

            // Get wallet's first address
            let addresses_resp = client
                .get(format!("{}/wallet/addresses", wallet_headless_url))
                .header("X-Wallet-Id", wallet_id)
                .send()
                .await
                .map_err(|e| format!("Failed to get wallet addresses: {}", e))?;

            let addresses: Value = addresses_resp
                .json()
                .await
                .map_err(|_| "Failed to parse addresses")?;

            let first_address = addresses
                .get("addresses")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .and_then(|a| a.as_str())
                .ok_or("Wallet has no addresses. Wait for it to sync.")?;

            // Get faucet balance
            let balance_resp = client
                .get(format!("{}/v1a/wallet/balance/", fullnode_url))
                .send()
                .await
                .map_err(|e| format!("Failed to get faucet balance: {}", e))?;

            let balance: Value = balance_resp
                .json()
                .await
                .map_err(|_| "Failed to parse faucet balance")?;

            let available = balance
                .get("balance")
                .and_then(|b| b.get("available"))
                .and_then(|a| a.as_i64())
                .unwrap_or(0);

            if available <= 0 {
                return Err("Faucet has no funds. Mine some blocks first.".to_string());
            }

            // Calculate amount
            let fund_amount = match amount {
                Some(a) => (a * 100.0) as i64,
                None => {
                    let ten_percent = available / 10;
                    ten_percent.max(100).min(10000)
                }
            };

            // Send from faucet
            let send_resp = client
                .post(format!("{}/v1a/wallet/send_tokens/", fullnode_url))
                .json(&json!({
                    "data": {
                        "inputs": [],
                        "outputs": [{
                            "address": first_address,
                            "value": fund_amount,
                        }]
                    }
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to send from faucet: {}", e))?;

            let text = send_resp.text().await.unwrap_or_default();
            Ok(format!(
                r#"{{"funded": true, "wallet_id": "{}", "amount": {}, "result": {}}}"#,
                wallet_id,
                fund_amount as f64 / 100.0,
                text
            ))
        }

        // Blockchain
        "get_blocks" => {
            let count = params.get("count").and_then(|v| v.as_i64()).unwrap_or(10) as usize;

            let status_resp = client
                .get(format!("{}/v1a/status/", fullnode_url))
                .send()
                .await
                .map_err(|e| format!("Failed to get status: {}", e))?;

            let status: Value = status_resp
                .json()
                .await
                .map_err(|_| "Failed to parse status")?;

            let height = status
                .get("dag")
                .and_then(|d| d.get("best_block"))
                .and_then(|b| b.get("height"))
                .and_then(|h| h.as_i64())
                .unwrap_or(0) as usize;

            let mut blocks = Vec::new();
            for i in (height.saturating_sub(count)..=height).rev() {
                if let Ok(resp) = client
                    .get(format!("{}/v1a/block_at_height?height={}", fullnode_url, i))
                    .send()
                    .await
                {
                    if let Ok(block) = resp.json::<Value>().await {
                        blocks.push(block);
                    }
                }
            }

            Ok(json!({"blocks": blocks, "currentHeight": height}).to_string())
        }

        "get_transaction" => {
            let tx_id = params
                .get("tx_id")
                .and_then(|v| v.as_str())
                .ok_or("tx_id is required")?;

            let resp = client
                .get(format!("{}/v1a/transaction?id={}", fullnode_url, tx_id))
                .send()
                .await
                .map_err(|e| format!("Failed to get transaction: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        // Utilities
        "quick_start" => {
            let mut results = Vec::new();

            // Start node
            match crate::start_node_internal(&state.app_state).await {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("Node: {}", e)),
            }

            // Wait a bit for node to be ready
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Start tx-mining-service (needed for wallet-headless transactions)
            match crate::start_tx_mining_internal(&state.app_state).await {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("TxMining: {}", e)),
            }

            // Wait for tx-mining-service to be ready
            tokio::time::sleep(Duration::from_secs(1)).await;

            // Start miner
            match crate::start_miner_internal(&state.app_state, None).await {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("Miner: {}", e)),
            }

            // Start headless
            let fn_url = state.fullnode_url.read().await;
            let txm_url = state.tx_mining_url.read().await;
            match crate::start_headless_internal(&state.app_state, Some(&fn_url), Some(&txm_url))
                .await
            {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("Headless: {}", e)),
            }

            Ok(results.join("\n"))
        }

        "quick_stop" => crate::stop_node_internal(&state.app_state).await,

        "get_full_status" => {
            let app_state = state.app_state.lock().await;
            let seeds = state.wallet_seeds.lock().await;

            let mut status = json!({
                "node": {
                    "running": app_state.node_running,
                    "pid": app_state.node_child_id,
                },
                "miner": {
                    "running": app_state.miner_running,
                    "pid": app_state.miner_child_id,
                },
                "headless": {
                    "running": app_state.headless_running,
                    "port": if app_state.headless_running { Some(8001) } else { None },
                },
                "tx_mining": {
                    "running": app_state.tx_mining_running,
                    "port": if app_state.tx_mining_running { Some(8002) } else { None },
                },
                "activeWallets": seeds.keys().collect::<Vec<_>>(),
            });

            drop(app_state);
            drop(seeds);

            // Try to get faucet balance
            if let Ok(resp) = client
                .get(format!("{}/v1a/wallet/balance/", fullnode_url))
                .send()
                .await
            {
                if let Ok(balance) = resp.json::<Value>().await {
                    status["faucetBalance"] = balance;
                }
            }

            Ok(status.to_string())
        }

        "reset_data" => {
            // Stop all services
            crate::stop_node_internal(&state.app_state).await?;

            // Clear wallet seeds
            state.wallet_seeds.lock().await.clear();

            // Remove data directory
            if let Some(data_dir) = dirs::home_dir() {
                let hathor_dir = data_dir.join(".hathor-forge");
                if hathor_dir.exists() {
                    let _ = std::fs::remove_dir_all(&hathor_dir);
                }
            }

            Ok("All data cleared. Start the node again to begin fresh.".to_string())
        }

        // Nano Contracts & Blueprints
        "list_blueprints" => {
            let resp = client
                .get(format!("{}/v1a/nano_contract/blueprints", fullnode_url))
                .send()
                .await
                .map_err(|e| format!("Failed to list blueprints: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_blueprint_info" => {
            let blueprint_id = params
                .get("blueprint_id")
                .and_then(|v| v.as_str())
                .ok_or("blueprint_id is required")?;

            let resp = client
                .get(format!(
                    "{}/v1a/nano_contract/blueprint?id={}",
                    fullnode_url, blueprint_id
                ))
                .send()
                .await
                .map_err(|e| format!("Failed to get blueprint info: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "publish_blueprint" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let code = params
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("code is required")?;
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("address is required")?;

            let resp = client
                .post(format!(
                    "{}/wallet/nano-contracts/create-on-chain-blueprint",
                    wallet_headless_url
                ))
                .header("X-Wallet-Id", wallet_id)
                .json(&json!({
                    "code": code,
                    "address": address,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to publish blueprint: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "create_nano_contract" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let blueprint_id = params
                .get("blueprint_id")
                .and_then(|v| v.as_str())
                .ok_or("blueprint_id is required")?;
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("address is required")?;
            let args = params.get("args").cloned().unwrap_or(json!([]));
            let actions = params.get("actions").cloned().unwrap_or(json!([]));

            let resp = client
                .post(format!(
                    "{}/wallet/nano-contracts/create",
                    wallet_headless_url
                ))
                .header("X-Wallet-Id", wallet_id)
                .json(&json!({
                    "blueprint_id": blueprint_id,
                    "address": address,
                    "data": {
                        "args": args,
                        "actions": actions,
                    },
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to create nano contract: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "execute_nano_contract" => {
            let wallet_id = params
                .get("wallet_id")
                .and_then(|v| v.as_str())
                .ok_or("wallet_id is required")?;
            let nc_id = params
                .get("nc_id")
                .and_then(|v| v.as_str())
                .ok_or("nc_id is required")?;
            let method = params
                .get("method")
                .and_then(|v| v.as_str())
                .ok_or("method is required")?;
            let address = params
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("address is required")?;
            let args = params.get("args").cloned().unwrap_or(json!([]));
            let actions = params.get("actions").cloned().unwrap_or(json!([]));

            let resp = client
                .post(format!(
                    "{}/wallet/nano-contracts/execute",
                    wallet_headless_url
                ))
                .header("X-Wallet-Id", wallet_id)
                .json(&json!({
                    "nc_id": nc_id,
                    "method": method,
                    "address": address,
                    "data": {
                        "args": args,
                        "actions": actions,
                    },
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to execute nano contract: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_nano_contract_state" => {
            let nc_id = params
                .get("nc_id")
                .and_then(|v| v.as_str())
                .ok_or("nc_id is required")?;

            let resp = client
                .get(format!(
                    "{}/v1a/nano_contract/state?id={}",
                    fullnode_url, nc_id
                ))
                .send()
                .await
                .map_err(|e| format!("Failed to get nano contract state: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_nano_contract_history" => {
            let nc_id = params
                .get("nc_id")
                .and_then(|v| v.as_str())
                .ok_or("nc_id is required")?;

            let resp = client
                .get(format!(
                    "{}/v1a/nano_contract/history?id={}",
                    fullnode_url, nc_id
                ))
                .send()
                .await
                .map_err(|e| format!("Failed to get nano contract history: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        "get_nano_contract_logs" => {
            let tx_id = params
                .get("tx_id")
                .and_then(|v| v.as_str())
                .ok_or("tx_id is required")?;

            let resp = client
                .get(format!(
                    "{}/v1a/nano_contract/logs?id={}",
                    fullnode_url, tx_id
                ))
                .send()
                .await
                .map_err(|e| format!("Failed to get nano contract logs: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            Ok(text)
        }

        // Service URL Configuration
        "get_service_urls" => Ok(json!({
            "fullnode_url": fullnode_url,
            "wallet_headless_url": wallet_headless_url,
            "tx_mining_url": _tx_mining_url,
        })
        .to_string()),

        "set_service_urls" => {
            if let Some(url) = params.get("fullnode_url").and_then(|v| v.as_str()) {
                *state.fullnode_url.write().await = url.to_string();
            }
            if let Some(url) = params.get("wallet_headless_url").and_then(|v| v.as_str()) {
                *state.wallet_headless_url.write().await = url.to_string();
            }
            if let Some(url) = params.get("tx_mining_url").and_then(|v| v.as_str()) {
                *state.tx_mining_url.write().await = url.to_string();
            }

            let fullnode_url = state.fullnode_url.read().await.clone();
            let wallet_headless_url = state.wallet_headless_url.read().await.clone();
            let tx_mining_url = state.tx_mining_url.read().await.clone();

            Ok(json!({
                "updated": true,
                "fullnode_url": fullnode_url,
                "wallet_headless_url": wallet_headless_url,
                "tx_mining_url": tx_mining_url,
            })
            .to_string())
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
