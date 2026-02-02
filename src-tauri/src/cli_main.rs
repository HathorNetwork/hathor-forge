use clap::{Parser, Subcommand};
use hathor_forge_lib::{
    start_headless_internal, start_miner_internal, start_node_internal,
    start_tx_mining_internal, stop_node_internal, AppState, MCP_SERVER_PORT,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Hathor Forge CLI — headless blockchain development environment
///
/// Run a local Hathor node, miner, wallet service, and MCP server without a GUI.
/// AI assistants can control the environment via MCP (port 9876 by default).
///
/// Examples:
///   hathor-forge-cli                        # Start MCP server only, control via MCP
///   hathor-forge-cli --start                # Start all local services + MCP
///   hathor-forge-cli --wallet-only          # Start only wallet-headless + MCP
///   hathor-forge-cli --fullnode-url http://mynode:8080  # Connect to external node
#[derive(Parser, Debug)]
#[command(name = "hathor-forge-cli", version, about)]
struct Cli {
    /// MCP server port
    #[arg(long, default_value_t = MCP_SERVER_PORT)]
    mcp_port: u16,

    /// Auto-start all local services (node + tx-mining + miner + wallet-headless)
    #[arg(long)]
    start: bool,

    /// Start only the wallet-headless service (+ MCP server)
    #[arg(long, conflicts_with = "start")]
    wallet_only: bool,

    /// Don't start the CPU miner (use with --start)
    #[arg(long)]
    no_miner: bool,

    /// Don't start the wallet-headless service (use with --start)
    #[arg(long)]
    no_wallet: bool,

    /// Don't start the tx-mining-service (use with --start)
    #[arg(long)]
    no_tx_mining: bool,

    /// External fullnode URL (skip starting local node)
    #[arg(long)]
    fullnode_url: Option<String>,

    /// External tx-mining-service URL (skip starting local tx-mining)
    #[arg(long)]
    tx_mining_url: Option<String>,

    /// Mining address (default: built-in dev wallet address)
    #[arg(long)]
    mining_address: Option<String>,

    /// Save current flags as default settings
    #[arg(long)]
    save_settings: bool,

    /// Path to settings file
    #[arg(long)]
    settings_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show current settings
    ShowSettings,
    /// Reset settings to defaults
    ResetSettings,
}

/// Persisted settings
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct Settings {
    mcp_port: Option<u16>,
    no_miner: Option<bool>,
    no_wallet: Option<bool>,
    no_tx_mining: Option<bool>,
    fullnode_url: Option<String>,
    tx_mining_url: Option<String>,
    mining_address: Option<String>,
    auto_start: Option<bool>,
    wallet_only: Option<bool>,
}

fn settings_path(custom: Option<&PathBuf>) -> PathBuf {
    if let Some(p) = custom {
        return p.clone();
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hathor-forge")
        .join("cli-settings.json")
}

fn load_settings(path: &PathBuf) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let sp = settings_path(cli.settings_file.as_ref());
    let saved = load_settings(&sp);

    // Handle subcommands
    match &cli.command {
        Some(Commands::ShowSettings) => {
            match std::fs::read_to_string(&sp) {
                Ok(contents) => {
                    println!("Settings file: {}", sp.display());
                    println!("{}", contents);
                }
                Err(_) => {
                    println!("No settings file found at {}", sp.display());
                    println!("Use --save-settings to create one.");
                }
            }
            return;
        }
        Some(Commands::ResetSettings) => {
            if sp.exists() {
                std::fs::remove_file(&sp).ok();
                println!("Settings reset (deleted {})", sp.display());
            } else {
                println!("No settings file to reset.");
            }
            return;
        }
        None => {}
    }

    // Merge CLI args with saved settings (CLI args take precedence)
    let mcp_port = if cli.mcp_port != MCP_SERVER_PORT {
        cli.mcp_port
    } else {
        saved.mcp_port.unwrap_or(MCP_SERVER_PORT)
    };
    let auto_start = cli.start || saved.auto_start.unwrap_or(false);
    let wallet_only = cli.wallet_only || saved.wallet_only.unwrap_or(false);
    let no_miner = cli.no_miner || saved.no_miner.unwrap_or(false);
    let no_wallet = cli.no_wallet || saved.no_wallet.unwrap_or(false);
    let no_tx_mining = cli.no_tx_mining || saved.no_tx_mining.unwrap_or(false);
    let fullnode_url = cli.fullnode_url.clone().or(saved.fullnode_url.clone());
    let tx_mining_url = cli.tx_mining_url.clone().or(saved.tx_mining_url.clone());
    let mining_address = cli.mining_address.clone().or(saved.mining_address.clone());

    // Save settings if requested
    if cli.save_settings {
        let settings = Settings {
            mcp_port: Some(mcp_port),
            no_miner: Some(no_miner),
            no_wallet: Some(no_wallet),
            no_tx_mining: Some(no_tx_mining),
            fullnode_url: fullnode_url.clone(),
            tx_mining_url: tx_mining_url.clone(),
            mining_address: mining_address.clone(),
            auto_start: Some(auto_start),
            wallet_only: Some(wallet_only),
        };
        match save_settings(&sp, &settings) {
            Ok(()) => eprintln!("Settings saved to {}", sp.display()),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    // Print banner
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║         Hathor Forge CLI v{}          ║", env!("CARGO_PKG_VERSION"));
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!();

    let state = Arc::new(Mutex::new(AppState::default()));

    // If using an external fullnode, mark node as "running" so internal checks pass
    if fullnode_url.is_some() {
        let mut s = state.lock().await;
        s.node_running = true;
        eprintln!("  External fullnode: {}", fullnode_url.as_ref().unwrap());
    }

    // Start services based on flags
    if auto_start || wallet_only {
        eprintln!("Starting services...");
        eprintln!();

        // Start local node (unless external)
        if fullnode_url.is_none() && !wallet_only {
            eprint!("  Node ............ ");
            match start_node_internal(&state).await {
                Ok(msg) => eprintln!("{}", msg),
                Err(e) => eprintln!("ERROR: {}", e),
            }
            // Wait for node readiness
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        // Start tx-mining-service (unless external or disabled)
        if tx_mining_url.is_none() && !no_tx_mining && !wallet_only {
            eprint!("  Tx-Mining ....... ");
            match start_tx_mining_internal(&state).await {
                Ok(msg) => eprintln!("{}", msg),
                Err(e) => eprintln!("ERROR: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Start miner (unless disabled)
        if !no_miner && !wallet_only {
            eprint!("  Miner ........... ");
            match start_miner_internal(&state, mining_address).await {
                Ok(msg) => eprintln!("{}", msg),
                Err(e) => eprintln!("ERROR: {}", e),
            }
        }

        // Start wallet-headless (unless disabled)
        if !no_wallet {
            eprint!("  Wallet-Headless . ");
            match start_headless_internal(&state).await {
                Ok(msg) => eprintln!("{}", msg),
                Err(e) => eprintln!("ERROR: {}", e),
            }
        }

        eprintln!();
    }

    // Start MCP server
    eprintln!("  MCP Server ...... http://127.0.0.1:{}", mcp_port);
    eprintln!();
    eprintln!("Press Ctrl+C to stop all services and exit.");

    let shutdown_state = state.clone();

    // Install Ctrl+C handler
    let ctrl_c = tokio::signal::ctrl_c();

    // Start MCP server
    let mcp_handle = tokio::spawn(async move {
        if let Err(e) = hathor_forge_lib::mcp::start_mcp_server(state, mcp_port).await {
            eprintln!("MCP server error: {}", e);
        }
    });

    // Wait for Ctrl+C
    tokio::select! {
        _ = ctrl_c => {
            eprintln!();
            eprintln!("Shutting down...");
        }
        _ = mcp_handle => {
            eprintln!("MCP server stopped unexpectedly.");
        }
    }

    // Graceful shutdown
    match stop_node_internal(&shutdown_state).await {
        Ok(msg) => eprintln!("  {}", msg),
        Err(e) => eprintln!("  Shutdown error: {}", e),
    }

    eprintln!("Done.");
}
