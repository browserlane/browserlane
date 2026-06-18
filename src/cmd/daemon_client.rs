use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::anyhow;
use serde_json::{Map, Value};

use super::daemon_cmd::wait_for_socket;
use crate::agent::ToolsCallResult;
use crate::daemon;
use crate::paths;

#[cfg(unix)]
use super::daemon_unix::set_sys_proc_attr;
#[cfg(windows)]
use super::daemon_windows::set_sys_proc_attr;

/// Sends a tool call to the daemon, auto-starting it if needed.
pub async fn daemon_call(
    tool_name: &str,
    args: Map<String, Value>,
    headless: bool,
) -> anyhow::Result<ToolsCallResult> {
    // First attempt.
    match daemon::call(tool_name, args.clone()).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            // If not a connection error, propagate.
            if !is_connection_error(&e) {
                return Err(e);
            }
        }
    }

    // Clean stale PID/socket files and auto-start the daemon.
    daemon::clean_stale();
    auto_start_daemon(headless)
        .await
        .map_err(|e| anyhow!("auto-start daemon: {e}"))?;

    // Retry.
    daemon::call(tool_name, args).await
}

/// Spawns a daemon process in the background and waits for it.
async fn auto_start_daemon(headless: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe().map_err(|e| anyhow!("find executable: {e}"))?;

    let mut args: Vec<String> = vec![
        "daemon".to_string(),
        "start".to_string(),
        "--_internal".to_string(),
        "--idle-timeout=30m".to_string(),
    ];
    if headless {
        args.push("--headless".to_string());
    }

    // Forward connect env vars to the spawned daemon.
    let (connect_url, connect_headers) = crate::connect_from_env();
    if !connect_url.is_empty() {
        args.push(format!("--connect={connect_url}"));
    }
    if let Some(headers) = &connect_headers {
        for (key, val) in headers.iter() {
            args.push(format!(
                "--connect-header={}: {}",
                key.as_str(),
                val.to_str().unwrap_or("")
            ));
        }
    }

    let mut cmd = Command::new(exe);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    set_sys_proc_attr(&mut cmd);

    cmd.spawn().map_err(|e| anyhow!("start daemon process: {e}"))?;

    let socket_path = paths::get_socket_path().map_err(|e| anyhow!("get socket path: {e}"))?;
    wait_for_socket(&socket_path.to_string_lossy(), Duration::from_secs(5)).await
}

/// Returns true if the error indicates the daemon is not running.
fn is_connection_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    for pattern in [
        "connect to daemon",
        "connection refused",
        "no such file or directory",
        "The system cannot find the path",
        "The system cannot find the file",
    ] {
        if msg.contains(pattern) {
            return true;
        }
    }
    false
}
