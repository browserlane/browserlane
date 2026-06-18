use std::sync::Arc;

use clap::{Arg, Command};

#[cfg(unix)]
use super::pipe_unix::wait_shutdown_signal;
#[cfg(windows)]
use super::pipe_windows::wait_shutdown_signal;
use crate::api;
use crate::browser;

pub fn serve_command() -> Command {
    Command::new("serve")
        .about("Start WebSocket proxy server for browser automation")
        // Hidden: library transport (ported as-is, still functional) but unadvertised —
        // browserlane is marketed as CLI + MCP. See PARITY-PLAN "library surface".
        .hide(true)
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .default_value("9515")
                .value_parser(clap::value_parser!(u16)),
        )
}

pub async fn run_serve(port: u16, headless: bool) {
    println!("Starting Browserlane proxy server on port {port}...");

    // Create router to manage browser sessions.
    let router = api::new_router(headless, "", None);

    let r_connect = Arc::clone(&router);
    let r_message = Arc::clone(&router);
    let r_close = Arc::clone(&router);

    let server = api::new_server(vec![
        api::with_port(port),
        api::with_on_connect(Arc::new(move |client| {
            let r = Arc::clone(&r_connect);
            Box::pin(async move { r.on_client_connect(client).await })
        })),
        api::with_on_message(Arc::new(move |client, msg| {
            let r = Arc::clone(&r_message);
            Box::pin(async move { r.on_client_message(&client, msg).await })
        })),
        api::with_on_close(Arc::new(move |client| {
            let r = Arc::clone(&r_close);
            Box::pin(async move { r.on_client_disconnect(&client).await })
        })),
    ]);

    if let Err(e) = server.start().await {
        eprintln!("Error starting server: {e}");
        std::process::exit(1);
    }

    println!("Server listening on ws://localhost:{}", server.port());
    println!("Press Ctrl+C to stop...");

    // Wait for signal.
    wait_shutdown_signal().await;

    println!("\nShutting down...");

    // Close all browser sessions, then sweep orphaned Chrome.
    router.close_all().await;
    browser::kill_orphaned_chrome_processes();

    server.stop();
}
