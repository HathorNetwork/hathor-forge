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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_rpc_request_deserializes() {
        let raw = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let req: JsonRpcRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.id, Some(json!(1)));
    }

    #[test]
    fn json_rpc_request_optional_params() {
        let raw = json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": "ping"
        });
        let req: JsonRpcRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(req.method, "ping");
        assert!(req.params.is_null());
    }

    #[test]
    fn json_rpc_response_serializes_result() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"ok": true})),
            error: None,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["result"]["ok"], true);
        // error field should be absent (skip_serializing_if)
        assert!(serialized.get("error").is_none());
    }

    #[test]
    fn json_rpc_response_serializes_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid request".to_string(),
                data: None,
            }),
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["error"]["code"], -32600);
        assert!(serialized.get("result").is_none());
    }

    #[test]
    fn mcp_tool_serializes_with_input_schema_rename() {
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({"type": "object"}),
        };
        let serialized = serde_json::to_value(&tool).unwrap();
        assert!(serialized.get("inputSchema").is_some());
        assert!(serialized.get("input_schema").is_none());
    }
}
