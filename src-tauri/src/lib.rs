pub mod config;
pub mod mcp;
pub mod platform;
pub mod services;
pub mod state;
pub mod tui;
pub mod types;

// Re-exports for backward compatibility
pub use config::*;
pub use platform::*;
pub use services::*;
pub use state::*;
pub use types::*;

// MCP Server port
pub const MCP_SERVER_PORT: u16 = 9876;

// ============================================================================
// Tauri GUI app (only compiled with tauri-app feature)
// ============================================================================

#[cfg(feature = "tauri-app")]
pub mod commands;

#[cfg(feature = "tauri-app")]
mod tauri_app;

#[cfg(feature = "tauri-app")]
pub use tauri_app::run;
