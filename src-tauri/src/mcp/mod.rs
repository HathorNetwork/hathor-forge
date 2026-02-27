//! MCP (Model Context Protocol) Server for Hathor Forge
//!
//! Provides an HTTP-based MCP server that allows AI assistants to control
//! the Hathor development environment.

pub mod handlers;
pub mod routes;
pub mod tools;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub use types::{McpSharedState, McpState};

use crate::SharedState;

/// Create the Axum router for the MCP server.
pub fn create_mcp_router(
    app_state: SharedState,
    fullnode_url: Option<String>,
    wallet_headless_url: Option<String>,
    tx_mining_url: Option<String>,
) -> Router {
    let mcp_state = Arc::new(McpState::new(
        app_state,
        fullnode_url,
        wallet_headless_url,
        tx_mining_url,
    ));

    Router::new()
        .route("/mcp", post(routes::handle_mcp_request))
        .route("/mcp/sse", get(routes::handle_sse))
        .route("/health", get(routes::handle_health))
        .with_state(mcp_state)
}

/// Start the MCP server on the specified port.
pub async fn start_mcp_server(
    app_state: SharedState,
    port: u16,
    fullnode_url: Option<String>,
    wallet_headless_url: Option<String>,
    tx_mining_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_mcp_router(app_state, fullnode_url, wallet_headless_url, tx_mining_url);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("MCP Server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
