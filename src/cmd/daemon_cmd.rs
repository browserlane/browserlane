use std::process::{Command as ProcCommand, Stdio};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::http::{HeaderMap, HeaderName, HeaderValue};

#[cfg(unix)]
use super::dial_unix::dial_socket;
#[cfg(windows)]
use super::dial_windows::dial_socket;
use super::output::print_json_value;
#[cfg(unix)]
use super::pipe_unix::wait_shutdown_signal;
#[cfg(windows)]
use super::pipe_windows::wait_shutdown_signal;
use crate::daemon;
use crate::errors::format_go_duration;
use crate::paths;

#[cfg(unix)]
use super::daemon_unix::set_sys_proc_attr;
#[cfg(windows)]
use super::daemon_windows::set_sys_proc_attr;

use daemon::process_exists;

pub fn daemon_command() -> Command {
    Command::new("daemon")
        .about("Manage the browserlane daemon (background browser process)")
        .subcommand(daemon_start_command())
        .subcommand(Command::new("stop").about("Stop the browserlane daemon"))
        .subcommand(Command::new("status").about("Show daemon status"))
}

fn daemon_start_command() -> Command {
    Command::new("start")
        .about("Start the browserlane daemon")
        .arg(
            Arg::new("foreground")
                .long("foreground")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("detach")
                .long("detach")
                .short('d')
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(Arg::new("idle-timeout").long("idle-timeout").default_value("30m"))
        .arg(
            Arg::new("_internal")
                .long("_internal")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(Arg::new("connect").long("connect").default_value(""))
        .arg(
            Arg::new("connect-header")
                .long("connect-header")
                .action(ArgAction::Append),
        )
}

/// Dispatches the `daemon` subcommands.
pub async fn run_daemon(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("start", sub)) => run_daemon_start(sub, headless).await,
        Some(("stop", _)) => run_daemon_stop().await,
        Some(("status", _)) => run_daemon_status(json_output).await,
        _ => {
            let _ = daemon_command().print_help();
            println!();
        }
    }
}

async fn run_daemon_start(sub: &ArgMatches, headless: bool) {
    let foreground = sub.get_flag("foreground");
    let internal = sub.get_flag("_internal");
    // Go validates --idle-timeout as a cobra Duration flag at parse time.
    let idle_timeout = match parse_duration_flag(
        "idle-timeout",
        sub.get_one::<String>("idle-timeout")
            .map(String::as_str)
            .unwrap_or("30m"),
    ) {
        Ok(d) => d,
        Err(msg) => {
            // TODO(P7-CLI): route through the central cobra usage renderer.
            eprintln!("Error: {msg}");
            std::process::exit(1);
        }
    };
    let connect_flag = sub
        .get_one::<String>("connect")
        .cloned()
        .unwrap_or_default();
    let header_flags: Vec<String> = sub
        .get_many::<String>("connect-header")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    if !foreground && !internal {
        daemonize(idle_timeout, &connect_flag, &header_flags, headless).await;
        return;
    }

    run_daemon_foreground(idle_timeout, &connect_flag, &header_flags, headless).await;
}

async fn run_daemon_stop() {
    if !daemon::is_running().await {
        println!("Daemon is not running.");
        return;
    }

    // Read PID before sending shutdown so we can wait for the process to exit.
    let pid = daemon::read_pid().unwrap_or(0);

    if let Err(e) = daemon::shutdown().await {
        eprintln!("Error stopping daemon: {e}");
        std::process::exit(1);
    }

    // Wait for the daemon process to fully exit (including Chrome cleanup).
    if pid > 0 {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if !process_exists(pid) {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    println!("Daemon stopped.");
}

async fn run_daemon_status(json_output: bool) {
    if !daemon::is_running().await {
        println!("Daemon is not running.");
        if json_output {
            print_json_value(&json!({ "running": false }));
        }
        return;
    }

    let status = match daemon::status().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error getting status: {e}");
            std::process::exit(1);
        }
    };

    if json_output {
        print_json_value(&json!({
            "running": true,
            "version": status.version,
            "pid": status.pid,
            "uptime": status.uptime,
            "socket": status.socket,
        }));
        return;
    }

    println!("browserlane daemon v{}", status.version);
    println!("status:   running");
    println!("pid:      {}", status.pid);
    println!("uptime:   {}", status.uptime);
    println!("socket:   {}", status.socket);
}

/// Merges CLI flags with env vars. Flags take precedence.
fn resolve_connect(connect_flag: &str, header_flags: &[String]) -> (String, Option<HeaderMap>) {
    let mut connect_url = connect_flag.to_string();
    if connect_url.is_empty() {
        connect_url = crate::connect_from_env().0;
    }

    let (_, mut headers) = crate::connect_from_env();

    if !header_flags.is_empty() {
        let h = headers.get_or_insert_with(HeaderMap::new);
        for hf in header_flags {
            if let Some((k, v)) = hf.split_once(':') {
                if let (Ok(name), Ok(val)) = (
                    k.trim().parse::<HeaderName>(),
                    v.trim().parse::<HeaderValue>(),
                ) {
                    h.insert(name, val);
                }
            }
        }
    }

    (connect_url, headers)
}

/// Starts the daemon in the current process.
async fn run_daemon_foreground(
    idle_timeout: Duration,
    connect_flag: &str,
    header_flags: &[String],
    headless: bool,
) {
    daemon::clean_stale();

    if daemon::is_running().await {
        eprintln!("Daemon is already running.");
        std::process::exit(1);
    }

    let screenshot_dir = paths::get_screenshot_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (connect_url, connect_headers) = resolve_connect(connect_flag, header_flags);

    let d = daemon::new(daemon::Options {
        version: crate::VERSION.to_string(),
        screenshot_dir,
        headless,
        idle_timeout,
        connect_url,
        connect_headers,
    });

    // Install signal handler for clean shutdown.
    let ds = std::sync::Arc::clone(&d);
    tokio::spawn(async move {
        wait_shutdown_signal().await;
        eprintln!("\nDaemon shutting down...");
        ds.shutdown();
    });

    let socket_path = paths::get_socket_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    eprintln!(
        "Daemon starting (pid {}, socket {})",
        std::process::id(),
        socket_path
    );

    if let Err(e) = d.run().await {
        eprintln!("Daemon error: {e}");
        std::process::exit(1);
    }
}

/// Spawns the daemon as a detached background process.
async fn daemonize(idle_timeout: Duration, connect_flag: &str, header_flags: &[String], headless: bool) {
    daemon::clean_stale();

    if daemon::is_running().await {
        println!("Daemon is already running.");
        return;
    }

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error finding executable: {e}");
            std::process::exit(1);
        }
    };

    let mut args: Vec<String> = vec![
        "daemon".to_string(),
        "start".to_string(),
        "--_internal".to_string(),
        format!("--idle-timeout={}", format_go_duration(idle_timeout)),
    ];
    if headless {
        args.push("--headless".to_string());
    }
    if !connect_flag.is_empty() {
        args.push(format!("--connect={connect_flag}"));
    }
    for h in header_flags {
        args.push(format!("--connect-header={h}"));
    }

    let mut cmd = ProcCommand::new(exe);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    set_sys_proc_attr(&mut cmd);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error starting daemon: {e}");
            std::process::exit(1);
        }
    };

    let socket_path = paths::get_socket_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Err(e) = wait_for_socket(&socket_path, Duration::from_secs(5)).await {
        eprintln!("Daemon failed to start: {e}");
        std::process::exit(1);
    }

    println!("Daemon started (pid {})", child.id());
}

/// Polls until the socket is connectable or the timeout elapses.
pub(crate) async fn wait_for_socket(socket_path: &str, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut interval = Duration::from_millis(50);

    while Instant::now() < deadline {
        if let Ok(conn) = dial_socket(socket_path, Duration::from_millis(500)).await {
            drop(conn);
            return Ok(());
        }
        sleep(interval).await;
        if interval < Duration::from_millis(500) {
            interval *= 2;
        }
    }

    Err(anyhow!(
        "socket not available after {}",
        format_go_duration(timeout)
    ))
}

/// Parses a duration the way Go's `time.ParseDuration` does, returning Go's exact
/// error text on failure (e.g. `time: invalid duration "bogus"`). This is what
/// cobra's Duration flag uses to validate `--timeout`/`--idle-timeout` at parse time.
pub(crate) fn go_parse_duration(s: &str) -> Result<Duration, String> {
    let orig = s;
    let invalid = || format!("time: invalid duration {}", go_quote(orig));

    let mut rest = s;
    let neg = match rest.as_bytes().first() {
        Some(b'-') => {
            rest = &rest[1..];
            true
        }
        Some(b'+') => {
            rest = &rest[1..];
            false
        }
        _ => false,
    };

    // Special case: "0" (optionally signed) is zero, with or without a unit.
    if rest == "0" {
        return Ok(Duration::ZERO);
    }
    if rest.is_empty() {
        return Err(invalid());
    }

    let mut total_nanos = 0f64;
    while !rest.is_empty() {
        let bytes = rest.as_bytes();
        // The next character must be [0-9.].
        if bytes[0] != b'.' && !bytes[0].is_ascii_digit() {
            return Err(invalid());
        }

        // Consume the number ([0-9]* then an optional .[0-9]*).
        let mut idx = 0;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        let pre = idx > 0;
        let mut post = false;
        if idx < bytes.len() && bytes[idx] == b'.' {
            idx += 1;
            let frac_start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            post = idx > frac_start;
        }
        if !pre && !post {
            return Err(invalid());
        }
        let num: f64 = rest[..idx].parse().map_err(|_| invalid())?;
        rest = &rest[idx..];

        // Consume the unit (runs until the next [0-9.] or end of string).
        let ubytes = rest.as_bytes();
        let mut ui = 0;
        while ui < ubytes.len() && ubytes[ui] != b'.' && !ubytes[ui].is_ascii_digit() {
            ui += 1;
        }
        if ui == 0 {
            return Err(format!("time: missing unit in duration {}", go_quote(orig)));
        }
        let unit = &rest[..ui];
        let mult = match unit {
            "ns" => 1.0,
            "us" | "µs" | "μs" => 1e3,
            "ms" => 1e6,
            "s" => 1e9,
            "m" => 60e9,
            "h" => 3600e9,
            _ => {
                return Err(format!(
                    "time: unknown unit {} in duration {}",
                    go_quote(unit),
                    go_quote(orig)
                ))
            }
        };
        rest = &rest[ui..];
        total_nanos += num * mult;
    }

    // Rust's Duration is unsigned; for the timeout flags a negative duration is a
    // pathological input. Use the magnitude so we never panic.
    let _ = neg;
    Ok(Duration::from_nanos(total_nanos as u64))
}

/// Validates a cobra Duration flag value, returning the exact cobra parse-error
/// text (`invalid argument "X" for "--flag" flag: <go error>`) on failure.
pub(crate) fn parse_duration_flag(flag: &str, value: &str) -> Result<Duration, String> {
    go_parse_duration(value)
        .map_err(|e| format!("invalid argument {} for \"--{flag}\" flag: {e}", go_quote(value)))
}

/// Quotes a string the way Go's `strconv.Quote` does for the common (printable
/// ASCII) inputs that reach the duration flags.
fn go_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
