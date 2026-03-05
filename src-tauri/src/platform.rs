use std::fs;
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;

use crate::config::HeadlessConfig;

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

    let binaries_dir = match std::env::var("HATHOR_FORGE_BINARIES_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
    };

    // hathor-core and tx-mining-service use onedir mode
    if name == "hathor-core" || name == "tx-mining-service" {
        let binary_name = format!("{}{}", name, exe_suffix);
        let onedir_path = binaries_dir
            .join(format!("{}-{}", name, target))
            .join(&binary_name);
        if onedir_path.exists() {
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

    // Fallback
    PathBuf::from("binaries").join(format!("{}-{}{}", name, target, exe_suffix))
}

/// Set platform-specific library path environment variable
pub fn set_library_path_env(cmd: &mut TokioCommand, internal_dir: &std::path::Path) {
    #[cfg(target_os = "macos")]
    cmd.env("DYLD_FALLBACK_LIBRARY_PATH", internal_dir);

    #[cfg(target_os = "linux")]
    cmd.env("LD_LIBRARY_PATH", internal_dir);

    #[cfg(target_os = "windows")]
    let _ = (cmd, internal_dir);
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

    let binaries_dir = match std::env::var("HATHOR_FORGE_BINARIES_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
    };

    // Try target-triple suffixed binary first
    let target_path = binaries_dir.join(format!("node-{}{}", target, exe_suffix));
    if target_path.exists() {
        return Ok(target_path);
    }

    // Try bare name
    let bare_path = binaries_dir.join(format!("node{}", exe_suffix));
    if bare_path.exists() {
        return Ok(bare_path);
    }

    // Dev fallback: use node from PATH
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from("node"));
    }

    Err(format!(
        "Bundled Node.js binary not found. Looked for {:?} and {:?}",
        target_path, bare_path
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

    PathBuf::from("wallet-headless-dist")
}

/// Detect network type from fullnode URL
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
  http_bind_address: 'localhost',
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

/// Kill any process using a specific port
pub fn kill_process_on_port(port: u16) {
    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()
        {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.lines() {
                if let Ok(pid_num) = pid.trim().parse::<u32>() {
                    let _ = Command::new("kill")
                        .args(["-9", &pid_num.to_string()])
                        .output();
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("netstat").args(["-ano", "-p", "TCP"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
                    if let Some(pid) = line.split_whitespace().last() {
                        let _ = Command::new("taskkill").args(["/PID", pid, "/F"]).output();
                    }
                }
            }
        }
    }
}

/// Kill a process by PID (graceful then force) — async version.
///
/// Uses `tokio::time::sleep` instead of `std::thread::sleep` so it does not
/// block the async runtime while waiting for the process to exit.
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
        eprintln!(
            "Process {} did not exit after SIGTERM, sending SIGKILL",
            pid
        );
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}

/// Kill a process by PID — synchronous version for non-async contexts
/// (e.g. the Tauri `RunEvent::Exit` handler).
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
        eprintln!(
            "Process {} did not exit after SIGTERM, sending SIGKILL",
            pid
        );
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}
