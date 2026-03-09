//! Shared process management abstraction.
//!
//! Provides helpers to eliminate the duplicated spawn/monitor/kill boilerplate
//! across service modules (node, miner, headless, tx-mining).

use crate::platform::kill_process;
use crate::state::{spawn_log_reader, AppState, LogBuffer, SharedState};

/// Set up stdout/stderr log readers for a spawned child process.
///
/// Pipes both streams into the shared [`LogBuffer`] tagged with `label`.
/// Returns the PID if the process is still alive, logging a warning otherwise.
pub fn setup_child_logging(
    child: &mut tokio::process::Child,
    log_buf: LogBuffer,
    label: &'static str,
) -> Option<u32> {
    let pid = child.id();
    if pid.is_none() {
        eprintln!("{} process exited immediately; no PID available", label);
    }

    if let Some(out) = child.stdout.take() {
        spawn_log_reader(out, log_buf.clone(), label);
    }
    if let Some(err) = child.stderr.take() {
        spawn_log_reader(err, log_buf, label);
    }

    pid
}

/// Spawn a background tokio task that waits for the child process to exit,
/// then calls `on_exit` to clear the running/PID fields in [`AppState`].
///
/// Takes ownership of the `Child`.
pub fn spawn_exit_monitor(
    mut child: tokio::process::Child,
    state: SharedState,
    on_exit: fn(&mut AppState),
) {
    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut guard = state.lock().await;
        on_exit(&mut guard);
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
