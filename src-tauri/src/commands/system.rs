use crate::*;
use std::sync::Arc;

// Graceful shutdown: stop all services in order, then allow exit
#[tauri::command]
pub(crate) async fn graceful_shutdown(
    state: tauri::State<'_, SharedState>,
    exit_flag: tauri::State<'_, Arc<std::sync::atomic::AtomicBool>>,
) -> Result<String, String> {
    let result = stop_node_internal(&state).await;
    exit_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    result
}
