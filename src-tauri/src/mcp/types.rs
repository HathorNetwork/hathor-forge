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
}

impl McpState {
    pub fn new(
        app_state: SharedState,
        fullnode_url: Option<String>,
        wallet_headless_url: Option<String>,
        tx_mining_url: Option<String>,
    ) -> Self {
        Self {
            app_state,
            wallet_seeds: Mutex::new(HashMap::new()),
            fullnode_url: RwLock::new(
                fullnode_url.unwrap_or_else(|| "http://127.0.0.1:8080".to_string()),
            ),
            wallet_headless_url: RwLock::new(
                wallet_headless_url.unwrap_or_else(|| "http://localhost:8001".to_string()),
            ),
            tx_mining_url: RwLock::new(
                tx_mining_url.unwrap_or_else(|| "http://localhost:8002".to_string()),
            ),
        }
    }
}

pub type McpSharedState = Arc<McpState>;
