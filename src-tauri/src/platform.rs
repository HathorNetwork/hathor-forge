use std::fs;
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;
use tracing::{debug, warn};

use crate::config::HeadlessConfig;

/// Get the directory containing the running executable.
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Get candidate directories where binaries might be located.
/// Checks (in order): HATHOR_FORGE_BINARIES_DIR env var, CARGO_MANIFEST_DIR/binaries (dev),
/// and the directory next to the running executable (production).
fn get_binaries_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Explicit env var override
    if let Ok(dir) = std::env::var("HATHOR_FORGE_BINARIES_DIR") {
        dirs.push(PathBuf::from(dir));
    }

    // 2. Development: relative to CARGO_MANIFEST_DIR
    let cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
    if cargo_dir.exists() {
        dirs.push(cargo_dir);
    }

    // 3. Production: Tauri places resources in Contents/Resources/ on macOS,
    //    next to the executable on Linux/Windows.
    if let Some(dir) = exe_dir() {
        // macOS: Contents/MacOS/../Resources/binaries
        #[cfg(target_os = "macos")]
        {
            let resources_binaries = dir.join("../Resources/binaries");
            if resources_binaries.exists() {
                dirs.push(resources_binaries);
            }
        }

        let exe_binaries = dir.join("binaries");
        if exe_binaries.exists() {
            dirs.push(exe_binaries);
        }

        // Windows: some installers place resources in a resources/ subdirectory
        #[cfg(target_os = "windows")]
        {
            let resources_binaries = dir.join("resources").join("binaries");
            if resources_binaries.exists() {
                dirs.push(resources_binaries);
            }
        }

        // Also check the exe directory itself
        dirs.push(dir);
    }

    dirs
}

/// Get the path to a binary (handles dev vs production, platform differences)
pub fn get_binary_path(name: &str) -> PathBuf {
    let target = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    } else {
        "x86_64-pc-windows-msvc"
    };

    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    let binaries_dirs = get_binaries_dirs();
    debug!(
        binary = name,
        target = target,
        candidate_dirs = ?binaries_dirs,
        "Resolving binary path"
    );

    for binaries_dir in &binaries_dirs {
        // hathor-core and tx-mining-service use onedir mode
        if name == "hathor-core" || name == "tx-mining-service" {
            let binary_name = format!("{}{}", name, exe_suffix);
            let onedir_path = binaries_dir
                .join(format!("{}-{}", name, target))
                .join(&binary_name);
            if onedir_path.exists() {
                debug!(path = ?onedir_path, "Found onedir binary");
                return onedir_path;
            }
        }

        // Single-file binaries with target triple suffix
        let dev_path = binaries_dir.join(format!("{}-{}{}", name, target, exe_suffix));
        if dev_path.exists() {
            return dev_path;
        }

        // Bare name (for Nix or PATH-style layouts)
        let bare_path = binaries_dir.join(format!("{}{}", name, exe_suffix));
        if bare_path.exists() {
            return bare_path;
        }
    }

    // Fallback: use first binaries_dir for error reporting
    warn!(
        binary = name,
        target = target,
        candidate_dirs = ?binaries_dirs,
        "Binary not found in any candidate directory, using fallback path"
    );
    let fallback_dir = binaries_dirs
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("binaries"));
    fallback_dir.join(format!("{}-{}{}", name, target, exe_suffix))
}

/// Set platform-specific library path environment variable so that bundled
/// shared libraries (in the PyInstaller `_internal` directory) can be found.
pub fn set_library_path_env(cmd: &mut TokioCommand, internal_dir: &std::path::Path) {
    #[cfg(target_os = "macos")]
    cmd.env("DYLD_FALLBACK_LIBRARY_PATH", internal_dir);

    #[cfg(target_os = "linux")]
    cmd.env("LD_LIBRARY_PATH", internal_dir);

    #[cfg(target_os = "windows")]
    {
        // On Windows, prepend the _internal dir to PATH so the loader finds DLLs there.
        let mut path = std::ffi::OsString::from(internal_dir);
        path.push(";");
        if let Some(existing) = std::env::var_os("PATH") {
            path.push(existing);
        }
        cmd.env("PATH", path);
    }
}

/// Set the PATH for cpuminer to find its bundled MinGW DLLs on Windows.
/// The DLLs are placed in a `cpuminer-deps` resource directory by the build script.
/// On non-Windows this is a no-op.
pub fn set_cpuminer_dll_path(cmd: &mut TokioCommand) {
    #[cfg(target_os = "windows")]
    {
        // Check candidate locations for cpuminer-deps/
        let candidates: Vec<PathBuf> = {
            let mut c = Vec::new();
            // Dev: next to Cargo.toml
            let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpuminer-deps");
            if dev_path.exists() {
                c.push(dev_path);
            }
            // Production: next to exe, or in resources/
            if let Some(dir) = exe_dir() {
                let p = dir.join("cpuminer-deps");
                if p.exists() {
                    c.push(p);
                }
                let p = dir.join("resources").join("cpuminer-deps");
                if p.exists() {
                    c.push(p);
                }
            }
            c
        };

        if let Some(deps_dir) = candidates.first() {
            debug!(path = ?deps_dir, "Adding cpuminer-deps to PATH");
            let mut path = std::ffi::OsString::from(deps_dir);
            path.push(";");
            if let Some(existing) = std::env::var_os("PATH") {
                path.push(existing);
            }
            cmd.env("PATH", path);
        }
    }

    // Suppress unused variable warning on non-Windows
    #[cfg(not(target_os = "windows"))]
    let _ = cmd;
}

/// Get the path to the bundled Node.js binary.
///
/// In production builds the binary is expected at
/// `src-tauri/binaries/node-{target-triple}` (or `.exe` on Windows).
/// During development, if no bundled binary is found we fall back to
/// the bare `node` command so the system PATH is used instead.
pub fn get_node_binary_path() -> Result<PathBuf, String> {
    let target = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    } else {
        "x86_64-pc-windows-msvc"
    };

    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    let binaries_dirs = get_binaries_dirs();

    let mut last_target_path = PathBuf::new();
    let mut last_bare_path = PathBuf::new();

    for binaries_dir in &binaries_dirs {
        // Try target-triple suffixed binary first
        let target_path = binaries_dir.join(format!("node-{}{}", target, exe_suffix));
        if target_path.exists() {
            return Ok(target_path);
        }
        last_target_path = target_path;

        // Try bare name
        let bare_path = binaries_dir.join(format!("node{}", exe_suffix));
        if bare_path.exists() {
            return Ok(bare_path);
        }
        last_bare_path = bare_path;
    }

    // Dev fallback: use node from PATH
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from("node"));
    }

    Err(format!(
        "Bundled Node.js binary not found. Looked for {:?} and {:?}",
        last_target_path, last_bare_path
    ))
}

/// Get the path to the wallet-headless-dist directory
pub fn get_headless_dist_path() -> PathBuf {
    if let Ok(dir) = std::env::var("HATHOR_FORGE_HEADLESS_DIR") {
        return PathBuf::from(dir);
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wallet-headless-dist");
    if dev_path.exists() {
        return dev_path;
    }

    // Production: Tauri places resources in Contents/Resources/ on macOS,
    // next to the executable on Linux/Windows.
    if let Some(dir) = exe_dir() {
        // macOS: Contents/MacOS/../Resources/wallet-headless-dist
        #[cfg(target_os = "macos")]
        {
            let resources_path = dir.join("../Resources/wallet-headless-dist");
            if resources_path.exists() {
                return resources_path;
            }
        }

        let exe_path = dir.join("wallet-headless-dist");
        if exe_path.exists() {
            return exe_path;
        }

        // Windows: some installers place resources in a resources/ subdirectory
        #[cfg(target_os = "windows")]
        {
            let resources_path = dir.join("resources").join("wallet-headless-dist");
            if resources_path.exists() {
                return resources_path;
            }
        }
    }

    PathBuf::from("wallet-headless-dist")
}

/// Detect network type from fullnode URL
#[allow(clippy::if_same_then_else)]
pub fn detect_network_from_url(url: &str) -> &'static str {
    let url_lower = url.to_lowercase();
    if url_lower.contains("mainnet") {
        "mainnet"
    } else if url_lower.contains("testnet") || url_lower.contains("playground") {
        "testnet"
    } else if url_lower.contains("localhost") || url_lower.contains("127.0.0.1") {
        "privatenet"
    } else {
        "privatenet"
    }
}

/// Generate wallet-headless config file
pub fn generate_headless_config(
    config: &HeadlessConfig,
    headless_dist_path: &std::path::Path,
    tx_mining_url: &str,
) -> Result<(), String> {
    let config_path = headless_dist_path.join("dist").join("config.js");

    let config_content = format!(
        r#"module.exports = {{
  http_bind_address: '127.0.0.1',
  http_port: {},
  network: '{}',
  server: '{}',
  txMiningUrl: '{}',
  seeds: {{}},
  allowPassphrase: false,
  confirmFirstAddress: false,
  tokenUid: '00',
  gapLimit: 20,
  connectionTimeout: 5000,
}}
"#,
        config.port, config.network, config.fullnode_url, tx_mining_url
    );

    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write headless config: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Windows console helpers for graceful child shutdown
// ---------------------------------------------------------------------------
//
// On Windows, GUI applications (Tauri) have no console. Without one, spawned
// child processes cannot receive CTRL_C/CTRL_BREAK signals for graceful
// shutdown.  By allocating a *hidden* console at startup and letting children
// inherit it (instead of setting `CREATE_NO_WINDOW`), we can later send
// `CTRL_C_EVENT` so hathor-core flushes its RocksDB before we force-kill.

#[cfg(windows)]
static CONSOLE_AVAILABLE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
mod win_console {
    type HWND = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;

    pub const CTRL_C_EVENT: DWORD = 0;
    pub const SW_HIDE: i32 = 0;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn AllocConsole() -> BOOL;
        pub fn GetConsoleWindow() -> HWND;
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(DWORD) -> BOOL>,
            add: BOOL,
        ) -> BOOL;
        pub fn GenerateConsoleCtrlEvent(ctrl_event: DWORD, process_group_id: DWORD) -> BOOL;
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn ShowWindow(hwnd: HWND, cmd_show: i32) -> BOOL;
    }
}

/// Allocate a hidden console so spawned children can receive `CTRL_C` for
/// graceful shutdown.  Call once at app startup.  No-op on non-Windows.
pub fn init_console_for_children() {
    #[cfg(windows)]
    {
        use std::sync::atomic::Ordering;
        unsafe {
            if win_console::AllocConsole() != 0 {
                // New console allocated — hide its window immediately.
                let hwnd = win_console::GetConsoleWindow();
                if !hwnd.is_null() {
                    win_console::ShowWindow(hwnd, win_console::SW_HIDE);
                }
                CONSOLE_AVAILABLE.store(true, Ordering::SeqCst);
                debug!("Allocated hidden console for child signal delivery");
            } else {
                // AllocConsole failed — we may already have a console (CLI).
                let hwnd = win_console::GetConsoleWindow();
                if !hwnd.is_null() {
                    CONSOLE_AVAILABLE.store(true, Ordering::SeqCst);
                }
            }
        }
    }
}

/// Send `CTRL_C` to all children sharing our console.  Used during shutdown
/// to give hathor-core a chance to flush its database.
/// No-op on non-Windows or when no console was allocated.
pub fn send_ctrl_c_to_children() {
    #[cfg(windows)]
    {
        if !CONSOLE_AVAILABLE.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        unsafe {
            // Ignore CTRL_C for ourselves — we are shutting down intentionally.
            win_console::SetConsoleCtrlHandler(None, 1);
            win_console::GenerateConsoleCtrlEvent(win_console::CTRL_C_EVENT, 0);
        }
    }
}

/// Apply platform-specific flags to hide console windows on Windows.
///
/// When a hidden console has been allocated via [`init_console_for_children`],
/// children inherit it silently and no flag is needed.  Otherwise falls back
/// to `CREATE_NO_WINDOW` to suppress visible console windows.
/// No-op on non-Windows platforms.
pub fn hide_console_window(cmd: &mut TokioCommand) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if !CONSOLE_AVAILABLE.load(std::sync::atomic::Ordering::SeqCst) {
            // No hidden console — fall back to suppressing the window.
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        // When a console IS available the child inherits it without any
        // creation flags, enabling CTRL_C delivery for graceful shutdown.
    }
    let _ = cmd; // suppress unused warning on non-Windows
}

/// Kill a process by PID (graceful then force) — async version.
///
/// Uses `tokio::time::sleep` instead of `std::thread::sleep` so it does not
/// block the async runtime while waiting for the process to exit.
///
/// On Windows this tree-kills the target and all descendants (`taskkill /T /F`).
/// Our services (hathor-core/tx-mining-service via PyInstaller, wallet-headless
/// via Node, cpuminer) are all spawned with `CREATE_NO_WINDOW`, so a graceful
/// `taskkill` that sends `WM_CLOSE` is a no-op — we go straight to force.
/// Without `/T` the PyInstaller bootloader's child Python process and any
/// grandchildren are orphaned, which is what was leaving hathor-core running
/// after the app window closed.
pub async fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        use std::process::Command;
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Ok(output) = Command::new("kill").args(["-0", &pid.to_string()]).output() {
                if !output.status.success() {
                    return;
                }
            } else {
                return;
            }
        }
        warn!(
            pid = pid,
            "Process did not exit after SIGTERM, sending SIGKILL"
        );
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Tree-kill the entire descendant chain. /T kills children, /F forces
        // termination without waiting for WM_CLOSE (which windowless services
        // never handle anyway).
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        // Give the OS a moment to tear the process tree down.
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let output = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if !stdout.contains(&pid.to_string()) {
                    return;
                }
            } else {
                return;
            }
        }
        warn!(pid = pid, "Process still present after taskkill /T /F");
    }
}

/// Kill a process by PID — synchronous version for non-async contexts
/// (e.g. the Tauri `RunEvent::Exit` handler).
///
/// See [`kill_process`] for the rationale behind the Windows tree-kill.
pub fn kill_process_sync(pid: u32) {
    #[cfg(unix)]
    {
        use std::process::Command;
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(output) = Command::new("kill").args(["-0", &pid.to_string()]).output() {
                if !output.status.success() {
                    return;
                }
            } else {
                return;
            }
        }
        warn!(
            pid = pid,
            "Process did not exit after SIGTERM, sending SIGKILL"
        );
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Tree-kill the entire descendant chain; see `kill_process` for why
        // we skip the graceful step on Windows.
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let output = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if !stdout.contains(&pid.to_string()) {
                    return;
                }
            } else {
                return;
            }
        }
        warn!(pid = pid, "Process still present after taskkill /T /F");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_binary_path_returns_path_with_name() {
        let path = get_binary_path("cpuminer");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("cpuminer"),
            "Path should contain binary name: {}",
            path_str
        );
    }

    #[test]
    fn get_binary_path_hathor_core_onedir() {
        let path = get_binary_path("hathor-core");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("hathor-core"),
            "Path should contain hathor-core: {}",
            path_str
        );
    }

    #[test]
    fn detect_network_from_url_mainnet() {
        assert_eq!(
            detect_network_from_url("https://mainnet.hathor.network"),
            "mainnet"
        );
    }

    #[test]
    fn detect_network_from_url_testnet() {
        assert_eq!(
            detect_network_from_url("https://testnet.hathor.network"),
            "testnet"
        );
    }

    #[test]
    fn detect_network_from_url_playground() {
        assert_eq!(
            detect_network_from_url("https://playground.hathor.network"),
            "testnet"
        );
    }

    #[test]
    fn detect_network_from_url_localhost() {
        assert_eq!(
            detect_network_from_url("http://localhost:8080"),
            "privatenet"
        );
        assert_eq!(
            detect_network_from_url("http://127.0.0.1:8080"),
            "privatenet"
        );
    }

    #[test]
    fn detect_network_from_url_unknown_defaults_privatenet() {
        assert_eq!(
            detect_network_from_url("http://some-host:8080"),
            "privatenet"
        );
    }

    #[test]
    fn get_node_binary_path_dev_fallback() {
        // In debug builds, should fall back to "node" when no binary is found
        if cfg!(debug_assertions) {
            let result = get_node_binary_path();
            assert!(result.is_ok());
        }
    }
}
