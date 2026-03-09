use super::commands::*;
use super::*;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;
use tracing::{error, info};

/// Tauri AppHandle-based event emitter for MCP → frontend communication.
struct TauriEventEmitter {
    app_handle: tauri::AppHandle,
}

impl mcp::EventEmitter for TauriEventEmitter {
    fn emit_event(&self, event: &str, payload: &str) {
        let _ = self.app_handle.emit(event, payload.to_string());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let state = Arc::new(Mutex::new(AppState::default())) as SharedState;
    let cleanup_state = state.clone();
    let mcp_state = state.clone();
    let exit_allowed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _exit_allowed_run = exit_allowed.clone();
    let exit_allowed_cmd = exit_allowed.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .manage(exit_allowed_cmd)
        .invoke_handler(tauri::generate_handler![
            start_node,
            stop_node,
            start_miner,
            stop_miner,
            get_node_status,
            get_miner_status,
            get_state,
            reset_data,
            get_wallet_addresses,
            get_fullnode_balance,
            send_tx,
            start_explorer_server,
            stop_explorer_server,
            start_headless,
            stop_headless,
            get_headless_status,
            start_tx_mining,
            stop_tx_mining,
            get_tx_mining_status,
            generate_seed,
            create_headless_wallet,
            get_headless_wallet_status,
            get_headless_wallet_balance,
            get_headless_wallet_addresses,
            headless_wallet_send_tx,
            close_headless_wallet,
            get_nano_contract_state,
            get_nano_contract_history,
            list_blueprints,
            get_blueprint_information,
            headless_wallet_create_nano_contract,
            headless_wallet_call_nano_contract_method,
            graceful_shutdown,
            get_mcp_config,
            get_local_ip,
        ])
        .setup(move |app| {
            let emitter = Box::new(TauriEventEmitter {
                app_handle: app.handle().clone(),
            });

            // Store AppHandle in AppState so the MCP/services layer can emit Tauri events.
            {
                let state = app.state::<SharedState>();
                let mut guard = state.blocking_lock();
                guard.app_handle = Some(app.handle().clone());
            }

            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
                let quit_item = MenuItemBuilder::new("Quit Hathor Forge")
                    .id("custom-quit")
                    .accelerator("CmdOrCtrl+Q")
                    .build(app)?;
                let app_submenu = SubmenuBuilder::new(app, "Hathor Forge")
                    .about(Some(AboutMetadata::default()))
                    .separator()
                    .services()
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator()
                    .item(&quit_item)
                    .build()?;
                let edit_submenu = SubmenuBuilder::new(app, "Edit")
                    .undo()
                    .redo()
                    .separator()
                    .cut()
                    .copy()
                    .paste()
                    .select_all()
                    .build()?;
                let window_submenu = SubmenuBuilder::new(app, "Window")
                    .minimize()
                    .separator()
                    .close_window()
                    .build()?;
                let menu = MenuBuilder::new(app)
                    .item(&app_submenu)
                    .item(&edit_submenu)
                    .item(&window_submenu)
                    .build()?;
                app.set_menu(menu)?;
            }

            tauri::async_runtime::spawn(async move {
                if let Err(e) = mcp::start_mcp_server(
                    mcp_state,
                    MCP_SERVER_PORT,
                    None,
                    None,
                    None,
                    Some(emitter),
                )
                .await
                {
                    error!(
                        service = "mcp",
                        port = MCP_SERVER_PORT,
                        "Failed to start MCP server: {}",
                        e
                    );
                }
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "custom-quit" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = cleanup_state.blocking_lock();

                if let Some(pid) = state.miner_child_id {
                    info!(service = "miner", pid = pid, "Cleaning up miner process");
                    kill_process_sync(pid);
                }

                if let Some(pid) = state.headless_child_id {
                    info!(
                        service = "wallet-headless",
                        pid = pid,
                        "Cleaning up wallet-headless process"
                    );
                    kill_process_sync(pid);
                }

                if let Some(pid) = state.tx_mining_child_id {
                    info!(
                        service = "tx-mining",
                        pid = pid,
                        "Cleaning up tx-mining-service process"
                    );
                    kill_process_sync(pid);
                }

                if let Some(pid) = state.node_child_id {
                    info!(service = "node", pid = pid, "Cleaning up node process");
                    kill_process_sync(pid);
                }
            }
        });
}
