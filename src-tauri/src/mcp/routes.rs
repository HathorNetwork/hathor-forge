//! Axum route handlers for the MCP HTTP server.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
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

/// CORS reverse proxy for the fullnode API.
///
/// Forwards requests from `/proxy/node/*path` to the fullnode, adding CORS
/// headers so browser dapps on different origins can access the API.
/// hathor-core sets `Access-Control-Allow-Origin: *` on responses but does
/// not handle OPTIONS preflight requests, which browsers require.
pub async fn handle_node_proxy(
    State(state): State<McpSharedState>,
    req: Request,
) -> Response<Body> {
    // Strip the /proxy/node prefix to get the downstream path + query
    let uri = req.uri();
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let downstream_path = path_and_query
        .strip_prefix("/proxy/node")
        .unwrap_or(path_and_query);

    let fullnode_url = state.fullnode_url.read().await.clone();
    let target_url = format!("{}{}", fullnode_url.trim_end_matches('/'), downstream_path);

    // Handle preflight OPTIONS requests directly
    if req.method() == axum::http::Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                "GET, POST, PUT, DELETE, OPTIONS",
            )
            .header(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                "content-type, x-prototype-version, x-requested-with",
            )
            .header(header::ACCESS_CONTROL_MAX_AGE, "604800")
            .body(Body::empty())
            .unwrap();
    }

    // Forward the request to the fullnode
    let method = req.method().clone();
    let mut builder = state.http_client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &target_url,
    );

    // Copy content-type header if present
    if let Some(ct) = req.headers().get(header::CONTENT_TYPE) {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("content-type", ct_str);
        }
    }

    // Forward request body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Body::from("Failed to read request body"))
                .unwrap();
        }
    };
    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes.to_vec());
    }

    // Execute the request
    let upstream_resp = match builder.send().await {
        Ok(r) => r,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
                ))
                .unwrap();
        }
    };

    // Build response with CORS headers
    let status = StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::OK);
    let content_type = upstream_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let resp_bytes = upstream_resp.bytes().await.unwrap_or_default();

    Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, PUT, DELETE, OPTIONS",
        )
        .header(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            "content-type, x-prototype-version, x-requested-with",
        )
        .header(header::CONTENT_TYPE, HeaderValue::from_str(&content_type).unwrap_or_else(|_| HeaderValue::from_static("application/json")))
        .body(Body::from(resp_bytes))
        .unwrap()
}
