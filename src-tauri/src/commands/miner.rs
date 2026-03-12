use crate::services::{
    miner::{start_miner_internal, stop_miner_internal},
    tx_mining::{start_tx_mining_internal, stop_tx_mining_internal},
};
use crate::state::SharedState;
use crate::types::TxMiningStatus;

// Start the CPU miner
#[tauri::command]
pub(crate) async fn start_miner(
    state: tauri::State<'_, SharedState>,
    config: Option<crate::config::MinerConfig>,
) -> Result<String, String> {
    let address = config.map(|c| c.address);
    start_miner_internal(&state, address).await
}

// Stop the CPU miner
#[tauri::command]
pub(crate) async fn stop_miner(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    stop_miner_internal(&state).await
}

// Start the tx-mining-service
#[tauri::command]
pub(crate) async fn start_tx_mining(
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    start_tx_mining_internal(&state).await
}

// Stop the tx-mining-service
#[tauri::command]
pub(crate) async fn stop_tx_mining(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    stop_tx_mining_internal(&state).await
}

// Get tx-mining-service status
#[tauri::command]
pub(crate) async fn get_tx_mining_status(
    state: tauri::State<'_, SharedState>,
) -> Result<TxMiningStatus, String> {
    let state_guard = state.lock().await;

    Ok(TxMiningStatus {
        running: state_guard.tx_mining_running,
        port: if state_guard.tx_mining_running {
            Some(state_guard.ports.tx_mining_api)
        } else {
            None
        },
    })
}
