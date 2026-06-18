use clap::{Arg, ArgAction, Command};
use tokio_tungstenite::tungstenite::http::HeaderMap;

#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(unix)]
use super::pipe_unix::{dup_fd, wait_shutdown_signal};
#[cfg(unix)]
use crate::api::{self, ClientTransport};
#[cfg(unix)]
use crate::browser;

pub fn pipe_command() -> Command {
    Command::new("pipe")
        .about("Run as a child process communicating via stdin/stdout pipes")
        // Hidden: library transport (ported as-is, still functional) but unadvertised —
        // browserlane is marketed as CLI + MCP. See PARITY-PLAN "library surface".
        .hide(true)
        .arg(Arg::new("connect").long("connect").default_value(""))
        .arg(
            Arg::new("connect-header")
                .long("connect-header")
                .action(ArgAction::Append),
        )
}

#[cfg(unix)]
pub async fn run_pipe(connect_url: String, connect_headers: Option<HeaderMap>, headless: bool) {
    // Save the real fd 1 for protocol output BEFORE redirecting.
    let protocol_fd = match dup_fd(1) {
        Ok(fd) => fd,
        Err(e) => {
            eprintln!("[pipe] Failed to dup stdout: {e}");
            std::process::exit(1);
        }
    };

    // Redirect fd 1 to fd 2 (stderr) so stray stdout writes don't corrupt the
    // protocol stream. Protocol output goes through `protocol_out` (the saved fd).
    // SAFETY: dup2 on valid fds.
    unsafe {
        libc::dup2(2, 1);
    }
    let protocol_out = unsafe {
        use std::os::unix::io::FromRawFd;
        std::fs::File::from_raw_fd(protocol_fd)
    };

    let router = api::new_router(headless, &connect_url, connect_headers);
    let client: Arc<dyn ClientTransport> = Arc::new(api::new_pipe_client_conn(protocol_out));

    // OnClientConnect blocks until Chrome is launched, BiDi connected, and
    // events subscribed — the client won't see messages until it's ready.
    router.on_client_connect(Arc::clone(&client)).await;

    // Send ready signal so the client knows it can start sending commands.
    let ready = json!({
        "method": "browserlane:lifecycle.ready",
        "params": { "version": crate::VERSION },
    });
    if client.send(&ready.to_string()).is_err() {
        eprintln!("[pipe] Failed to send ready signal");
        std::process::exit(1);
    }

    // Read commands from stdin line by line in a background task.
    let reader_router = Arc::clone(&router);
    let reader_client = Arc::clone(&client);
    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }
            reader_router.on_client_message(&reader_client, line).await;
        }
    });

    // Wait for stdin EOF or a shutdown signal.
    tokio::select! {
        _ = reader => {}
        _ = wait_shutdown_signal() => {}
    }

    // Clean up — close this process's session, then sweep orphaned Chrome.
    router.on_client_disconnect(&client).await;
    router.close_all().await;
    browser::kill_orphaned_chrome_processes();

    client.close();
}

#[cfg(windows)]
pub async fn run_pipe(_connect_url: String, _connect_headers: Option<HeaderMap>, _headless: bool) {
    // `pipe` is a hidden library transport (stdin/stdout BiDi). browserlane ships
    // CLI + MCP and provides no client libraries, so pipe mode — which relies on
    // POSIX fd redirection — is unsupported on Windows.
    eprintln!("pipe mode is not supported on Windows");
    std::process::exit(1);
}
