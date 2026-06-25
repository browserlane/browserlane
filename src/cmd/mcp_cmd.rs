use std::io::IsTerminal;

use clap::{Arg, Command};

use super::examples::{examples, PROG};
use crate::agent::{new_server, ServerOptions};
use crate::paths;

pub fn mcp_command() -> Command {
    Command::new("mcp")
        .about("Start MCP server (stdio JSON-RPC for LLM agents)")
        .long_about(
            "Start the Model Context Protocol (MCP) server.\n\n\
             This runs a JSON-RPC 2.0 server over stdin/stdout, designed for integration\n\
             with LLM agents like Claude Code.\n\n\
             The server provides browser automation tools:\n\
             \x20 - browser_start: Start a browser session\n\
             \x20 - browser_navigate: Go to a URL\n\
             \x20 - browser_click: Click an element\n\
             \x20 - browser_type: Type into an element\n\
             \x20 - browser_screenshot: Capture the page\n\
             \x20 - browser_find: Find element info\n\
             \x20 - browser_evaluate: Execute JavaScript\n\
             \x20 - browser_stop: Stop the browser\n\
             \x20 - browser_get_text: Get page/element text\n\
             \x20 - browser_get_url: Get current URL\n\
             \x20 - browser_get_title: Get page title\n\
             \x20 - browser_get_html: Get page/element HTML\n\
             \x20 - browser_find_all: Find all matching elements\n\
             \x20 - browser_wait: Wait for element state\n\
             \x20 - browser_hover: Hover over an element\n\
             \x20 - browser_select: Select a dropdown option\n\
             \x20 - browser_scroll: Scroll the page\n\
             \x20 - browser_keys: Press keys\n\
             \x20 - browser_new_page: Open a new page\n\
             \x20 - browser_list_pages: List open pages\n\
             \x20 - browser_switch_page: Switch pages\n\
             \x20 - browser_close_page: Close a page",
        )
        .arg(
            Arg::new("screenshot-dir")
                .long("screenshot-dir")
                .help("Directory for saving screenshots (default: ~/Pictures/browserlane, use \"\" to disable)"),
        )
        .after_help(examples(&[
            (
                "mcp",
                "Run directly for testing (configure in Claude Code via: claude mcp add ...)",
            ),
            (
                "mcp --screenshot-dir ./screenshots",
                "Custom screenshot directory",
            ),
            (
                "mcp --screenshot-dir \"\"",
                "Disable screenshot file saving (inline only)",
            ),
            (
                &format!(
                    "echo '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"capabilities\":{{}}}}}}' | {PROG} mcp"
                ),
                "Test with echo",
            ),
        ]))
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
