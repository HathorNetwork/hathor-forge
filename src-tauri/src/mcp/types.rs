//! MCP protocol types and server state.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

use crate::SharedState;

// ============================================================================
// JSON-RPC Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ============================================================================
// MCP Tool Definition
// ============================================================================

#[derive(Debug, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

// ============================================================================
// MCP Server State
// ============================================================================

pub struct McpState {
    pub app_state: SharedState,
    pub wallet_seeds: Mutex<HashMap<String, String>>,
    pub fullnode_url: RwLock<String>,
    pub wallet_headless_url: RwLock<String>,
    pub tx_mining_url: RwLock<String>,
    /// Shared HTTP client with connection pooling and default timeout.
    pub http_client: reqwest::Client,
}

impl McpState {
    pub fn new(
        app_state: SharedState,
        fullnode_url: Option<String>,
        wallet_headless_url: Option<String>,
        tx_mining_url: Option<String>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            app_state,
            wallet_seeds: Mutex::new(HashMap::new()),
            fullnode_url: RwLock::new(fullnode_url.unwrap_or_else(|| {
                format!(
                    "http://127.0.0.1:{}",
                    crate::config::DEFAULT_FULLNODE_API_PORT
                )
            })),
            wallet_headless_url: RwLock::new(wallet_headless_url.unwrap_or_else(|| {
                format!(
                    "http://localhost:{}",
                    crate::config::DEFAULT_WALLET_HEADLESS_PORT
                )
            })),
            tx_mining_url: RwLock::new(tx_mining_url.unwrap_or_else(|| {
                format!(
                    "http://localhost:{}",
                    crate::config::DEFAULT_TX_MINING_API_PORT
                )
            })),
            http_client,
        }
    }
}

pub type McpSharedState = Arc<McpState>;
