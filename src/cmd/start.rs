use std::process::{Command as ProcCommand, Stdio};
use std::time::{Duration, Instant};

use clap::{Arg, Command};
use serde_json::Map;
use tokio::time::sleep;

use super::daemon_cmd::wait_for_socket;
use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};
use crate::{daemon, paths};

#[cfg(unix)]
use super::daemon_unix::set_sys_proc_attr;
#[cfg(windows)]
use super::daemon_windows::set_sys_proc_attr;

pub fn start_command() -> Command {
    Command::new("start")
        .about("Start a browser session")
        .long_about(
            "Start a browser session. Without arguments, launches a local browser.\n\
With a URL argument, connects to a remote BiDi WebSocket endpoint.\n\n\
If no URL is given, checks BROWSERLANE_CONNECT_URL env var before falling\n\
back to a local browser launch.\n\n\
Set BROWSERLANE_CONNECT_API_KEY to send an Authorization: Bearer header.",
        )
        .arg(Arg::new("url").num_args(0..=1))
}

pub async fn run_start(url: Option<String>, headless: bool, json_output: bool) {
    // Determine connect URL: arg > env > local.
    let connect_url = match url {
        Some(u) => u,
        None => crate::connect_from_env().0,
    };

    if connect_url.is_empty() {
        // Local launch — just ensure the daemon is running (lazy browser launch).
        match daemon_call("browser_start", Map::new(), headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    // Remote connect — stop any existing daemon and start fresh with --connect.
    if daemon::is_running().await {
        let pid = daemon::read_pid().unwrap_or(0);
        if let Err(e) = daemon::shutdown().await {
            eprintln!("Error stopping existing daemon: {e}");
            std::process::exit(1);
        }
        // Wait for the daemon process to fully exit.
        if pid > 0 {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if !daemon::process_exists(pid) {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    daemon::clean_stale();

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error finding executable: {e}");
            std::process::exit(1);
        }
    };

    let mut daemon_args: Vec<String> = vec![
        "daemon".to_string(),
        "start".to_string(),
        "--_internal".to_string(),
        "--idle-timeout=30m".to_string(),
        format!("--connect={connect_url}"),
    ];
    if headless {
        daemon_args.push("--headless".to_string());
    }

    let (_url, env_headers) = crate::connect_from_env();
    if let Some(headers) = &env_headers {
        for (key, val) in headers.iter() {
            daemon_args.push(format!("--connect-header={}: {}", key.as_str(), val.to_str().unwrap_or("")));
        }
    }

    let mut child_cmd = ProcCommand::new(&exe);
    child_cmd
        .args(&daemon_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    set_sys_proc_attr(&mut child_cmd);

    let child = match child_cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error starting daemon: {e}");
            std::process::exit(1);
        }
    };
    let pid = child.id();

    let socket_path = paths::get_socket_path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    if let Err(e) = wait_for_socket(&socket_path, Duration::from_secs(5)).await {
        eprintln!("Daemon failed to start: {e}");
        std::process::exit(1);
    }

    println!("Connected to {connect_url} (daemon pid {pid})");
    let _ = json_output;
}
