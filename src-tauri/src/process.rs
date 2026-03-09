//! Shared process management abstraction.
//!
//! Provides helpers to eliminate the duplicated spawn/monitor/kill boilerplate
//! across service modules (node, miner, headless, tx-mining).

use crate::platform::kill_process;
use crate::state::{spawn_log_reader, AppState, LogBuffer, SharedState};
use tauri::Emitter;

/// Set up stdout/stderr log readers for a spawned child process.
///
/// Pipes both streams into the shared [`LogBuffer`] tagged with `label`,
/// and optionally emits Tauri events for each line if `app_handle` is provided.
/// Returns the PID if the process is still alive, logging a warning otherwise.
pub fn setup_child_logging(
    child: &mut tokio::process::Child,
    log_buf: LogBuffer,
    label: &'static str,
    app_handle: Option<tauri::AppHandle>,
) -> Option<u32> {
    let pid = child.id();
    if pid.is_none() {
        eprintln!("{} process exited immediately; no PID available", label);
    }

    if let Some(out) = child.stdout.take() {
        spawn_log_reader(out, log_buf.clone(), label, app_handle.clone());
    }
    if let Some(err) = child.stderr.take() {
        spawn_log_reader(err, log_buf, label, app_handle);
    }

    pid
}

/// Spawn a background tokio task that waits for the child process to exit,
/// calls `on_exit` to clear the running/PID fields in [`AppState`],
/// and emits a Tauri termination event if an AppHandle is available.
///
/// `get_current_pid` reads the current PID from state — if it no longer
/// matches the PID of the monitored child, a new process has been started
/// and we must not clear its state.
///
/// Takes ownership of the `Child`.
pub fn spawn_exit_monitor(
    mut child: tokio::process::Child,
    state: SharedState,
    on_exit: fn(&mut AppState),
    get_current_pid: fn(&AppState) -> Option<u32>,
    terminated_event: &'static str,
) {
    let monitored_pid = child.id();
    tokio::spawn(async move {
        let status = child.wait().await;
        let exit_code = status.map(|s| s.code()).ok().flatten();
        let mut guard = state.lock().await;
        // Only clear state if this is still the current process.
        // If the PID changed, a new process was started — don't touch its state.
        let current_pid = get_current_pid(&guard);
        if current_pid == monitored_pid {
            on_exit(&mut guard);
            if let Some(ref handle) = guard.app_handle {
                let _ = handle.emit(terminated_event, exit_code);
            }
        }
    });
}

/// Stop a service by extracting its PID from state and killing the process.
///
/// `is_running` checks whether the service is currently running.
/// `take_pid` clears the running flag and returns the PID.
///
/// Returns a user-friendly message.
pub async fn stop_service(
    state: &SharedState,
    service_name: &str,
    is_running: fn(&AppState) -> bool,
    take_pid: fn(&mut AppState) -> Option<u32>,
) -> Result<String, String> {
    let pid = {
        let mut guard = state.lock().await;
        if !is_running(&guard) {
            return Ok(format!("{} is not running", service_name));
        }
        take_pid(&mut guard)
    };

    if let Some(pid) = pid {
        kill_process(pid).await;
    }

    Ok(format!("{} stopped", service_name))
}
