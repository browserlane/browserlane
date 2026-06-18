use std::io::IsTerminal;

use clap::{Arg, Command};

use crate::agent::{new_server, ServerOptions};
use crate::paths;

pub fn mcp_command() -> Command {
    Command::new("mcp")
        .about("Start MCP server (stdio JSON-RPC for LLM agents)")
        .arg(
            Arg::new("screenshot-dir")
                .long("screenshot-dir")
                .help("Directory for saving screenshots (default: ~/Pictures/browserlane, use \"\" to disable)"),
        )
}

pub async fn run_mcp(screenshot_dir: Option<String>) {
    // If running in a terminal, print helpful info to stderr.
    if std::io::stdin().is_terminal() {
        eprintln!("browserlane MCP server v{}", crate::VERSION);
        eprintln!("This server communicates via JSON-RPC over stdin/stdout.");
        eprintln!("It's meant to be run by an MCP client (e.g., Claude Desktop).");
        eprintln!();

        // Show Chrome for Testing status.
        match (paths::get_chrome_executable(), paths::get_chromedriver_path()) {
            (Ok(chrome), Ok(driver)) => {
                eprintln!("Chrome: {}", chrome.display());
                eprintln!("Chromedriver: {}", driver.display());
            }
            _ => {
                eprintln!("Chrome for Testing: not installed");
                eprintln!("Run 'bl install' to download Chrome for Testing and chromedriver.");
            }
        }

        eprintln!();
        eprintln!("Waiting for client connection on stdin...");
    }

    // Resolve the screenshot dir: an explicit flag (even "") wins; otherwise the
    // platform default (mirrors Go's `cmd.Flags().Changed("screenshot-dir")`).
    let screenshot_dir = match screenshot_dir {
        Some(d) => d,
        None => match paths::get_screenshot_dir() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                eprintln!("Warning: could not determine default screenshot directory: {e}");
                String::new()
            }
        },
    };

    let (connect_url, connect_headers) = crate::connect_from_env();

    let mut server = new_server(
        crate::VERSION,
        ServerOptions {
            screenshot_dir,
            connect_url,
            connect_headers,
        },
    );

    // Run the stdio loop, but also handle SIGTERM so Chrome is cleaned up even if
    // stdin is never closed. Go installs a SIGTERM-only handler here (unlike its
    // other commands, which also catch SIGINT), so we match that and leave SIGINT
    // to its default disposition.
    let run_result = tokio::select! {
        result = server.run() => Some(result),
        _ = wait_sigterm() => None,
    };

    server.close().await;

    match run_result {
        Some(Err(e)) => {
            eprintln!("MCP server error: {e}");
            std::process::exit(1);
        }
        None => std::process::exit(0), // SIGTERM
        Some(Ok(())) => {}             // stdin EOF — clean exit
    }
}

/// Resolves when SIGTERM is received. Mirrors Go's `signal.Notify(ch,
/// syscall.SIGTERM)` — SIGTERM only (SIGINT keeps its default disposition).
async fn wait_sigterm() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    }
    #[cfg(not(unix))]
    {
        std::future::pending::<()>().await;
    }
}
