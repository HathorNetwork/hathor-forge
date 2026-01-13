//! MCP (Model Context Protocol) Server for Hathor Forge
//!
//! Provides an HTTP-based MCP server that allows AI assistants to control
//! the Hathor development environment.

use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::SharedState;

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpTool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

// ============================================================================
// MCP Server State
// ============================================================================

pub struct McpState {
    app_state: SharedState,
    wallet_seeds: Mutex<HashMap<String, String>>,
}

impl McpState {
    pub fn new(app_state: SharedState) -> Self {
        Self {
            app_state,
            wallet_seeds: Mutex::new(HashMap::new()),
        }
    }
}

pub type McpSharedState = Arc<McpState>;

// ============================================================================
// Tool Definitions
// ============================================================================

fn get_tools() -> Vec<McpTool> {
    vec![
        // Node Management
        McpTool {
            name: "start_node".to_string(),
            description: "Start the Hathor fullnode. This must be running before mining or wallet operations.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "stop_node".to_string(),
            description: "Stop the Hathor fullnode and all related services (miner, wallet headless).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_node_status".to_string(),
            description: "Get the current status of the Hathor fullnode including block height and network info.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Miner Management
        McpTool {
            name: "start_miner".to_string(),
            description: "Start the CPU miner. The node must be running first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Mining reward address (uses node's wallet address if not provided)"
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "stop_miner".to_string(),
            description: "Stop the CPU miner.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_miner_status".to_string(),
            description: "Get the current status of the CPU miner.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Wallet Service
        McpTool {
            name: "start_wallet_service".to_string(),
            description: "Start the wallet-headless service for multi-wallet support. Node must be running first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "stop_wallet_service".to_string(),
            description: "Stop the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_wallet_service_status".to_string(),
            description: "Get the status of the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Wallet Operations
        McpTool {
            name: "generate_seed".to_string(),
            description: "Generate a new 24-word BIP39 seed phrase for wallet creation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "create_wallet".to_string(),
            description: "Create a new wallet. Generates a seed if not provided.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "Unique identifier for the wallet"
                    },
                    "seed": {
                        "type": "string",
                        "description": "24-word BIP39 seed phrase (generated if not provided)"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_seed".to_string(),
            description: "Retrieve the seed phrase for a wallet created in this session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_status".to_string(),
            description: "Get the sync status of a wallet (statusCode 3 = Ready).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_balance".to_string(),
            description: "Get the balance of a wallet (available and locked HTR in cents).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_addresses".to_string(),
            description: "Get the addresses of a wallet.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "send_from_wallet".to_string(),
            description: "Send HTR from a wallet to an address.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to send from"
                    },
                    "address": {
                        "type": "string",
                        "description": "Destination Hathor address"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send"
                    }
                },
                "required": ["wallet_id", "address", "amount"]
            }),
        },
        McpTool {
            name: "close_wallet".to_string(),
            description: "Close a wallet and remove it from the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        // Faucet
        McpTool {
            name: "get_faucet_balance".to_string(),
            description: "Get the balance of the fullnode's built-in wallet (faucet).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "send_from_faucet".to_string(),
            description: "Send HTR from the fullnode's built-in wallet (faucet) to an address.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Destination Hathor address"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send"
                    }
                },
                "required": ["address", "amount"]
            }),
        },
        McpTool {
            name: "fund_wallet".to_string(),
            description: "Send HTR from the faucet to a wallet. Auto-determines address and reasonable amount.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to fund"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send (auto-calculated if not provided)"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        // Blockchain
        McpTool {
            name: "get_blocks".to_string(),
            description: "Get recent blocks from the blockchain.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "Number of blocks to retrieve (default: 10)"
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "get_transaction".to_string(),
            description: "Get details of a specific transaction.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tx_id": {
                        "type": "string",
                        "description": "Transaction ID (hash)"
                    }
                },
                "required": ["tx_id"]
            }),
        },
        // Utilities
        McpTool {
            name: "quick_start".to_string(),
            description: "Quickly start the full environment: node, miner, and wallet service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "quick_stop".to_string(),
            description: "Stop all services: node, miner, and wallet service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_full_status".to_string(),
            description: "Get comprehensive status of all services, balances, and active wallets.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "reset_data".to_string(),
            description: "Reset all blockchain data and stop all services. USE WITH CAUTION.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

// ============================================================================
// Tool Execution
// ============================================================================

async fn execute_tool(state: &McpState, name: &str, params: &Value) -> Result<String, String> {
    let client = reqwest::Client::new();

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

            match client.get("http://127.0.0.1:8080/v1a/status/").send().await {
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

        // Wallet Service
        "start_wallet_service" => crate::start_headless_internal(&state.app_state).await,

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
                .post("http://localhost:8001/start")
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
            Ok(json!({
                "success": result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                "wallet_id": wallet_id,
                "seed_stored": true,
                "message": if seed.is_some() { "Wallet created with provided seed" } else { "Wallet created with generated seed (use get_wallet_seed to retrieve)" }
            }).to_string())
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
                .get("http://localhost:8001/wallet/status")
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
                .get("http://localhost:8001/wallet/balance")
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
                .get("http://localhost:8001/wallet/addresses")
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
                .post("http://localhost:8001/wallet/simple-send-tx")
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
                .post("http://localhost:8001/wallet/stop")
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
                .get("http://127.0.0.1:8080/v1a/wallet/balance/")
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
                .post("http://127.0.0.1:8080/v1a/wallet/send_tokens/")
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
                .get("http://localhost:8001/wallet/addresses")
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
                .get("http://127.0.0.1:8080/v1a/wallet/balance/")
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
                .post("http://127.0.0.1:8080/v1a/wallet/send_tokens/")
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
                .get("http://127.0.0.1:8080/v1a/status/")
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
                    .get(format!(
                        "http://127.0.0.1:8080/v1a/block_at_height?height={}",
                        i
                    ))
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
                .get(format!(
                    "http://127.0.0.1:8080/v1a/transaction?id={}",
                    tx_id
                ))
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

            // Start miner
            match crate::start_miner_internal(&state.app_state, None).await {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("Miner: {}", e)),
            }

            // Start headless
            match crate::start_headless_internal(&state.app_state).await {
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
                "activeWallets": seeds.keys().collect::<Vec<_>>(),
            });

            drop(app_state);
            drop(seeds);

            // Try to get faucet balance
            if let Ok(resp) = reqwest::Client::new()
                .get("http://127.0.0.1:8080/v1a/wallet/balance/")
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

        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn handle_mcp_request(
    State(state): State<McpSharedState>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "hathor-forge",
                    "version": "1.0.0"
                }
            })),
            error: None,
        },

        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({})),
            error: None,
        },

        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "tools": get_tools()
            })),
            error: None,
        },

        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            match execute_tool(&state, tool_name, &arguments).await {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": result
                        }]
                    })),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {}", e)
                        }],
                        "isError": true
                    })),
                    error: None,
                },
            }
        }

        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({})),
            error: None,
        },

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        },
    };

    Json(response)
}

async fn handle_sse(
    State(_state): State<McpSharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // For now, just send periodic keepalive events
    let stream = stream::unfold((), |_| async {
        tokio::time::sleep(Duration::from_secs(30)).await;
        Some((Ok(Event::default().comment("keepalive")), ()))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// ============================================================================
// Router
// ============================================================================

pub fn create_mcp_router(app_state: SharedState) -> Router {
    let mcp_state = Arc::new(McpState::new(app_state));

    Router::new()
        .route("/mcp", post(handle_mcp_request))
        .route("/mcp/sse", get(handle_sse))
        .route("/health", get(handle_health))
        .with_state(mcp_state)
}

/// Start the MCP server on the specified port
pub async fn start_mcp_server(
    app_state: SharedState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_mcp_router(app_state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("MCP Server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
