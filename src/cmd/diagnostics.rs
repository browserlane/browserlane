use std::sync::Arc;

use clap::{Arg, Command};
use serde_json::json;
use tokio::io::AsyncBufReadExt;

use crate::bidi;
use crate::browser;
use crate::paths;
use crate::process;

use super::output::print_json_value;

/// Returns the program name shown in help, usage, and the launch screen.
///
/// This is the basename of argv0 (like Go's `filepath.Base(os.Args[0])`), but
/// lowercased and with a trailing `.exe` removed: on case-insensitive
/// filesystems (macOS, Windows) the one real `bl` binary is reachable as `BL`,
/// `Bl`, `bL`, … and argv0 preserves whatever the user typed. Lowercasing
/// collapses those back to the canonical `bl` so the UI never echoes a mis-cased
/// name. A genuinely different basename (a rename or symlink, only possible on a
/// case-sensitive filesystem) is still reflected.
///
/// On Windows the `.exe` suffix is stripped so the name renders as `bl` no matter
/// how the binary was invoked: PowerShell passes argv0 with the extension
/// (`bl.exe`) while cmd passes it bare (`bl`). Only `.exe` is removed, so a
/// genuine dotted rename is left intact.
pub fn prog_name() -> String {
    std::env::args()
        .next()
        .map(|arg0| {
            std::path::Path::new(&arg0)
                .file_name()
                .map(|f| {
                    let name = f.to_string_lossy().to_lowercase();
                    name.strip_suffix(".exe").map(str::to_string).unwrap_or(name)
                })
                .unwrap_or_else(|| arg0.to_lowercase())
        })
        .unwrap_or_default()
}

// ---- command definitions (registered in main.go's AddCommand order) ----

pub fn version_command() -> Command {
    Command::new("version").about("Print the version number")
}

pub fn paths_command() -> Command {
    Command::new("paths").about("Print browser and cache paths")
}

pub fn is_installed_command() -> Command {
    Command::new("is-installed")
        .about("Check if Chrome and chromedriver are installed (exit 0 = yes, exit 1 = no)")
}

pub fn install_command() -> Command {
    Command::new("install").about("Download Chrome for Testing and chromedriver")
}

pub fn launch_test_command() -> Command {
    Command::new("launch-test").about("Launch browser via chromedriver and print BiDi WebSocket URL")
}

pub fn ws_test_command() -> Command {
    Command::new("ws-test")
        .about("Test WebSocket connection (type messages, see echoes)")
        .arg(Arg::new("url").required(true).num_args(1))
}

pub fn bidi_test_command() -> Command {
    Command::new("bidi-test").about("Launch browser, connect via BiDi, send session.status")
}

// ---- command runners ----

pub fn run_version(json_output: bool) {
    if json_output {
        print_json_value(&json!({ "version": crate::VERSION }));
        return;
    }
    println!("{} v{}", prog_name(), crate::VERSION);
}

pub fn run_paths(json_output: bool) {
    let cache = paths::get_cache_dir();
    let chrome = paths::get_chrome_executable();
    let chromedriver = paths::get_chromedriver_path();

    if json_output {
        // Unresolved paths serialize as `null` (e.g. Chrome not yet installed),
        // so a consumer can distinguish "present" from "missing" structurally.
        print_json_value(&json!({
            "cache_dir": cache.as_ref().ok().map(|p| p.display().to_string()),
            "chrome": chrome.as_ref().ok().map(|p| p.display().to_string()),
            "chromedriver": chromedriver.as_ref().ok().map(|p| p.display().to_string()),
        }));
        return;
    }

    match cache {
        Ok(dir) => println!("Cache directory: {}", dir.display()),
        Err(e) => println!("Cache directory: error: {e}"),
    }
    match chrome {
        Ok(p) => println!("Chrome: {}", p.display()),
        Err(_) => println!("Chrome: not found"),
    }
    match chromedriver {
        Ok(p) => println!("Chromedriver: {}", p.display()),
        Err(_) => println!("Chromedriver: not found"),
    }
}

pub fn run_is_installed(json_output: bool) {
    let status = browser::install_status();
    let chromedriver = status.chromedriver;
    let version = status.chrome_version; // Option<String>; consumed below.
    let chrome = version.is_some();
    let installed = chrome && chromedriver;

    if json_output {
        // Enriched for agents: per-binary presence plus Chrome's version (`null`
        // when Chrome is absent). `installed` stays the top-level yes/no, so
        // existing consumers reading just that key are unaffected.
        print_json_value(&json!({
            "installed": installed,
            "chrome": chrome,
            "chromedriver": chromedriver,
            "version": version,
        }));
    } else {
        // Humans were left with a silent prompt and only the exit code to read;
        // give them a precise answer — the version when present, and exactly
        // which binary is missing otherwise. Agents/scripts use --json or `$?`.
        let p = prog_name();
        match (version, chromedriver) {
            (Some(v), true) if !v.is_empty() => {
                println!("Chrome {v} and chromedriver are installed.")
            }
            (Some(_), true) => println!("Chrome and chromedriver are installed."),
            (Some(_), false) => println!(
                "Chrome is installed, but chromedriver is not. Run \"{p} install\" to download it."
            ),
            (None, true) => println!(
                "chromedriver is installed, but Chrome is not. Run \"{p} install\" to download it."
            ),
            (None, false) => println!(
                "Chrome and chromedriver are not installed. Run \"{p} install\" to download them."
            ),
        }
    }

    // Exit-code contract is unchanged: 0 = installed, 1 = not.
    if !installed {
        std::process::exit(1);
    }
}

pub async fn run_install(json_output: bool) {
    match browser::install().await {
        Ok(result) => {
            if json_output {
                print_json_value(&json!({
                    "chrome": result.chrome_path,
                    "chromedriver": result.chromedriver_path,
                    "version": result.version,
                }));
                return;
            }
            println!("Installation complete!");
            println!("Chrome: {}", result.chrome_path);
            println!("Chromedriver: {}", result.chromedriver_path);
            println!("Version: {}", result.version);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn run_launch_test(headless: bool) {
    let result = match browser::launch(browser::LaunchOptions {
        headless,
        port: 0,
        verbose: false,
    })
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    println!("Session ID: {}", result.session_id);
    println!("BiDi WebSocket: {}", result.web_socket_url);
    println!("Press Ctrl+C to stop...");

    // Wait for signal, then cleanup.
    process::wait_for_signal();
    let _ = result.close().await;
}

pub async fn run_ws_test(url: String) {
    println!("Connecting to {url}...");

    let conn = match bidi::connect(&url).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    println!("Connected! Type messages (Ctrl+C to quit):");

    // Read responses in the background.
    let reader_conn = Arc::clone(&conn);
    tokio::spawn(async move {
        loop {
            match reader_conn.receive().await {
                Ok(msg) => println!("< {msg}"),
                Err(_) => return,
            }
        }
    });

    // Read input and send.
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Err(e) = conn.send(&line).await {
            eprintln!("Send error: {e}");
            break;
        }
        println!("> {line}");
    }

    let _ = conn.close().await;
}

pub async fn run_bidi_test() {
    println!("[1/5] Launching chromedriver...");
    let launch_result = match browser::launch(browser::LaunchOptions {
        headless: true,
        port: 0,
        verbose: true,
    })
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error launching browser: {e}");
            std::process::exit(1);
        }
    };
    println!("       Chromedriver started on port {}", launch_result.port);
    println!("       Session ID: {}", launch_result.session_id);

    println!("[2/5] WebDriver session created with BiDi enabled");
    println!("       WebSocket URL: {}", launch_result.web_socket_url);

    println!("[3/5] Connecting to BiDi WebSocket...");
    let conn = match bidi::connect(&launch_result.web_socket_url).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Error connecting: {e}");
            std::process::exit(1);
        }
    };
    println!("       Connected!");

    println!("[4/5] Sending BiDi command: session.status");
    let mut client = bidi::Client::new(Arc::clone(&conn));
    client.set_verbose(true);

    let status = match client.session_status().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    println!("[5/5] Parsed response:");
    println!("       Ready: {}", status.ready);
    println!("       Message: {}", status.message);

    println!();
    println!("Test complete!");

    // Deferred cleanup (LIFO): connection first, then the launch result.
    let _ = conn.close().await;
    let _ = launch_result.close().await;
}
