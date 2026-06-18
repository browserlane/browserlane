use std::sync::Arc;

use clap::{Arg, Command};
use tokio::io::AsyncBufReadExt;

use crate::bidi;
use crate::browser;
use crate::paths;
use crate::process;

/// Returns the program name (basename of argv0), matching Go's
/// `filepath.Base(os.Args[0])`.
pub fn prog_name() -> String {
    std::env::args()
        .next()
        .map(|arg0| {
            std::path::Path::new(&arg0)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or(arg0)
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

pub fn run_version() {
    println!("{} v{}", prog_name(), crate::VERSION);
}

pub fn run_paths() {
    match paths::get_cache_dir() {
        Ok(dir) => println!("Cache directory: {}", dir.display()),
        Err(e) => println!("Cache directory: error: {e}"),
    }

    match paths::get_chrome_executable() {
        Ok(p) => println!("Chrome: {}", p.display()),
        Err(_) => println!("Chrome: not found"),
    }

    match paths::get_chromedriver_path() {
        Ok(p) => println!("Chromedriver: {}", p.display()),
        Err(_) => println!("Chromedriver: not found"),
    }
}

pub fn run_is_installed() {
    if !browser::is_installed() {
        std::process::exit(1);
    }
}

pub async fn run_install() {
    match browser::install().await {
        Ok(result) => {
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
