use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::bidi;
use crate::log;
use crate::paths;
use crate::process;

#[cfg(unix)]
use super::launcher_unix::{
    kill_by_pid, kill_process_group, platform_chrome_args, set_proc_group, wait_for_process_dead,
};
#[cfg(windows)]
use super::launcher_windows::{
    kill_by_pid, kill_process_group, platform_chrome_args, set_proc_group, wait_for_process_dead,
};

/// Options for launching the browser.
#[derive(Debug, Default, Clone)]
pub struct LaunchOptions {
    pub headless: bool,
    /// Chromedriver port, 0 = auto-select.
    pub port: u16,
    /// Show chromedriver output.
    pub verbose: bool,
}

/// Result of launching the browser via chromedriver.
pub struct LaunchResult {
    /// Non-None when the session was created via BiDi (no HTTP fallback).
    pub bidi_conn: Option<Arc<bidi::Connection>>,
    /// Set when the session was created via the HTTP fallback.
    pub web_socket_url: String,
    pub session_id: String,
    pub chromedriver_cmd: process::Browser,
    pub port: u16,
    /// Chrome temp profile dir — cleaned up on `close()`.
    pub user_data_dir: String,
}

/// Response from creating a new session.
#[derive(Debug, Deserialize)]
struct SessionResponse {
    value: SessionValue,
}

#[derive(Debug, Deserialize)]
struct SessionValue {
    #[serde(rename = "sessionId", default)]
    session_id: String,
    #[serde(default)]
    capabilities: serde_json::Map<String, Value>,
}

/// Starts chromedriver and creates a BiDi session.
pub async fn launch(opts: LaunchOptions) -> anyhow::Result<LaunchResult> {
    log::debug("launching browser");

    let chromedriver_path =
        paths::get_chromedriver_path().map_err(|_| anyhow!("chromedriver not found"))?;
    log::debug("found chromedriver");

    let chrome_path = paths::get_chrome_executable()
        .map_err(|_| anyhow!("Chrome not found"))?
        .to_string_lossy()
        .into_owned();
    log::debug("found chrome");

    let port = if opts.port == 0 {
        find_available_port().map_err(|e| anyhow!("failed to find available port: {e}"))?
    } else {
        opts.port
    };
    log::debug("using port");

    // Start chromedriver as a process-group leader so we can kill all children.
    let mut cmd = Command::new(&chromedriver_path);
    cmd.arg(format!("--port={port}"));
    set_proc_group(&mut cmd);

    if opts.verbose {
        println!("       ------- chromedriver -------");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("failed to start chromedriver: {e}"))?;

    // When verbose, stream chromedriver's stdout/stderr with a line prefix
    // matching Go's prefixWriter ("       ").
    if opts.verbose {
        let gate = Arc::new(Mutex::new(()));
        if let Some(out) = child.stdout.take() {
            pump_prefixed(out, Arc::clone(&gate));
        }
        if let Some(err) = child.stderr.take() {
            pump_prefixed(err, Arc::clone(&gate));
        }
    }

    let child = Arc::new(Mutex::new(child));

    // Track for cleanup.
    process::track(&child);

    // Wait for chromedriver to be ready.
    let base_url = format!("http://localhost:{port}");
    if let Err(e) = wait_for_chromedriver(&base_url, Duration::from_secs(10)).await {
        let _ = child.lock().unwrap().kill();
        return Err(anyhow!("chromedriver failed to start: {e}"));
    }

    if opts.verbose {
        println!("       ----------------------------");
    }

    // Try BiDi session.new first (direct WebSocket, no HTTP round-trip).
    let ws_url = format!("ws://localhost:{port}/session");
    match bidi::connect(&ws_url).await {
        Ok(conn) => {
            let conn = Arc::new(conn);
            let client = bidi::Client::new(Arc::clone(&conn));
            let caps = build_capabilities(&chrome_path, opts.headless);
            match client.session_new(caps).await {
                Ok(result) => {
                    let user_data_dir = result
                        .capabilities
                        .get("userDataDir")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    log::info("browser launched via BiDi session.new");
                    return Ok(LaunchResult {
                        bidi_conn: Some(conn),
                        // The endpoint we connected to. Populated (not left empty)
                        // so diagnostics like `launch-test` can surface it; live
                        // consumers use `bidi_conn` above and ignore this.
                        web_socket_url: ws_url,
                        session_id: result.session_id,
                        chromedriver_cmd: child,
                        port,
                        user_data_dir,
                    });
                }
                Err(_) => {
                    log::debug("BiDi session.new failed, falling back to HTTP");
                    let _ = conn.close().await;
                }
            }
        }
        Err(_) => {
            log::debug("BiDi WebSocket connect failed, falling back to HTTP");
        }
    }

    // Fallback: HTTP POST /session (original path).
    let (session_id, http_ws_url, user_data_dir) =
        match create_session(&base_url, &chrome_path, opts.headless, opts.verbose).await {
            Ok(v) => v,
            Err(e) => {
                let _ = child.lock().unwrap().kill();
                return Err(anyhow!("failed to create session: {e}"));
            }
        };
    log::info("browser launched via HTTP");

    Ok(LaunchResult {
        bidi_conn: None,
        web_socket_url: http_ws_url,
        session_id,
        chromedriver_cmd: child,
        port,
        user_data_dir,
    })
}

/// Streams a child output pipe to stdout with a per-line "       " prefix,
/// serialized through a shared gate so stdout and stderr don't interleave
/// mid-line (mirrors Go's single shared prefixWriter).
fn pump_prefixed<R: Read + Send + 'static>(reader: R, gate: Arc<Mutex<()>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match buf.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let _guard = gate.lock().unwrap();
                    print!("       {line}");
                    let _ = std::io::stdout().flush();
                }
            }
        }
    })
}

/// Finds an available TCP port.
fn find_available_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

/// Waits for chromedriver to be ready.
async fn wait_for_chromedriver(base_url: &str, timeout: Duration) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(resp) = client.get(format!("{base_url}/status")).send().await {
            if resp.status() == reqwest::StatusCode::OK {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow!("timeout waiting for chromedriver"))
}

/// Returns the standard Chrome launch arguments.
fn chrome_args(headless: bool) -> Vec<String> {
    let mut args: Vec<String> = [
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-infobars",
        "--disable-blink-features=AutomationControlled",
        "--disable-crash-reporter",
        "--disable-background-networking",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-breakpad",
        "--disable-component-extensions-with-background-pages",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-dev-shm-usage",
        "--disable-extensions",
        "--disable-notifications",
        "--disable-features=TranslateUI,PasswordLeakDetection",
        "--disable-hang-monitor",
        "--disable-ipc-flooding-protection",
        "--disable-popup-blocking",
        "--disable-prompt-on-repost",
        "--disable-renderer-backgrounding",
        "--disable-sync",
        "--enable-features=NetworkService,NetworkServiceInProcess",
        "--force-color-profile=srgb",
        "--metrics-recording-only",
        "--password-store=basic",
        "--use-mock-keychain",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    args.extend(platform_chrome_args());
    if headless {
        args.push("--headless=new".to_string());
    }
    args
}

/// Returns the capabilities map for BiDi session.new / HTTP POST /session.
fn build_capabilities(chrome_path: &str, headless: bool) -> Value {
    json!({
        "alwaysMatch": {
            "browserName": "chrome",
            "webSocketUrl": true,
            "unhandledPromptBehavior": {
                "default": "ignore"
            },
            "goog:chromeOptions": {
                "binary": chrome_path,
                "args": chrome_args(headless),
                "excludeSwitches": ["enable-automation", "enable-logging"],
                "prefs": {
                    "credentials_enable_service": false,
                    "profile.password_manager_enabled": false,
                    "profile.password_manager_leak_detection": false,
                    "profile.default_content_setting_values.notifications": 2
                }
            }
        }
    })
}

/// Creates a new WebDriver session with BiDi enabled via HTTP.
async fn create_session(
    base_url: &str,
    chrome_path: &str,
    headless: bool,
    verbose: bool,
) -> anyhow::Result<(String, String, String)> {
    let req_body = json!({ "capabilities": build_capabilities(chrome_path, headless) });
    let json_body = serde_json::to_vec(&req_body)?;

    if verbose {
        println!("       ------- POST /session -------");
        println!("       --> {}", String::from_utf8_lossy(&json_body));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/session"))
        .header("content-type", "application/json")
        .body(json_body)
        .send()
        .await?;

    let status = resp.status();
    if status != reqwest::StatusCode::OK && status != reqwest::StatusCode::CREATED {
        return Err(anyhow!("failed to create session: HTTP {}", status.as_u16()));
    }

    let resp_body = resp.bytes().await?;

    if verbose {
        println!("       <-- {}", String::from_utf8_lossy(&resp_body));
        println!("       ------------------------------");
    }

    let sess_resp: SessionResponse = serde_json::from_slice(&resp_body)?;

    let ws_url = sess_resp
        .value
        .capabilities
        .get("webSocketUrl")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("webSocketUrl not found in session capabilities"))?
        .to_string();

    let user_data_dir = sess_resp
        .value
        .capabilities
        .get("userDataDir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok((sess_resp.value.session_id, ws_url, user_data_dir))
}

impl LaunchResult {
    /// Terminates the chromedriver session and process.
    pub async fn close(&self) -> anyhow::Result<()> {
        log::debug("closing browser");

        let pid = self.chromedriver_cmd.lock().unwrap().id();
        kill_process_tree(pid as i32);
        let _ = self.chromedriver_cmd.lock().unwrap().wait();
        process::untrack(&self.chromedriver_cmd);

        if !self.user_data_dir.is_empty() {
            log::debug("removing Chrome user data dir");
            let _ = std::fs::remove_dir_all(&self.user_data_dir);
        }

        Ok(())
    }
}

/// Kills a process and all its descendants using process-group kill.
fn kill_process_tree(pid: i32) {
    kill_process_group(pid);
    kill_by_pid(pid); // fallback: kill root directly if pgid lookup failed
    wait_for_process_dead(pid, Duration::from_secs(2));
}

/// Finds and kills Chrome/chromedriver processes that have been orphaned
/// (reparented to init/launchd).
pub fn kill_orphaned_chrome_processes() {
    let patterns = ["chromedriver", "Chrome for Testing"];

    for pattern in patterns {
        let output = match Command::new("pgrep").args(["-f", pattern]).output() {
            Ok(o) if o.status.success() => o.stdout,
            _ => continue,
        };

        for line in String::from_utf8_lossy(&output).split('\n') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let pid: i32 = match line.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Check if this process's parent is 1 (orphaned).
            let ppid_out = match Command::new("ps")
                .args(["-o", "ppid=", "-p", &pid.to_string()])
                .output()
            {
                Ok(o) => o.stdout,
                Err(_) => continue,
            };
            if let Ok(ppid) = String::from_utf8_lossy(&ppid_out).trim().parse::<i32>() {
                if ppid == 1 {
                    kill_process_group(pid);
                    kill_by_pid(pid);
                }
            }
        }
    }
}

/// Removes Chrome temp directories left behind by previous crashed runs. Only
/// deletes directories whose mtime is older than `min_age` so concurrent
/// sibling processes' live profiles are never touched.
pub fn cleanup_orphaned_chrome_temp_dirs(min_age: Duration) {
    let tmp_dir = std::env::temp_dir();
    let prefixes = [
        "com.google.chrome.for.testing.",
        "org.chromium.Chromium.scoped_dir.",
    ];
    let cutoff = std::time::SystemTime::now() - min_age;

    let entries = match std::fs::read_dir(&tmp_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let mtime = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if mtime > cutoff {
            continue;
        }
        if std::fs::remove_dir_all(entry.path()).is_ok() {
            count += 1;
        }
    }

    if count > 0 {
        log::debug("cleaned up orphaned Chrome temp dirs");
    }
}
