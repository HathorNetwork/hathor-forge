//! Axum route handlers for the MCP HTTP server.

use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Json,
};
use futures_util::stream::{self, Stream};
use serde_json::json;
use std::{convert::Infallible, time::Duration};

use super::handlers::execute_tool;
use super::tools::get_tools;
use super::types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpSharedState};

/// Handle incoming MCP JSON-RPC requests.
/// Notifications (methods starting with "notifications/") return 204 No Content.
pub async fn handle_mcp_request(
    State(state): State<McpSharedState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // JSON-RPC notifications have no id and expect no response.
    if request.method.starts_with("notifications/") {
        return (StatusCode::NO_CONTENT, "").into_response();
    }

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
                },
                "instructions": include_str!("instructions.md")
            })),
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

    Json(response).into_response()
}

/// Handle SSE connections for MCP event streaming.
pub async fn handle_sse(
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

/// Health check endpoint.
pub async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
