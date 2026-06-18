use std::time::Duration;

use crate::paths;

use super::dial;
use super::pidfile::read_pid;

#[cfg(unix)]
use super::process_unix::process_exists;
#[cfg(windows)]
use super::process_windows::process_exists;

/// Checks if a daemon is currently running. Verifies both that the PID file
/// references a live process and that the socket is connectable.
pub async fn is_running() -> bool {
    let pid = match read_pid() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if pid == 0 {
        return false;
    }
    if !process_exists(pid) {
        return false;
    }
    let socket_path = match paths::get_socket_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    socket_connectable(&socket_path.to_string_lossy()).await
}

/// Tests if the daemon socket accepts connections.
async fn socket_connectable(socket_path: &str) -> bool {
    dial(socket_path, Duration::from_secs(2)).await.is_ok()
}
