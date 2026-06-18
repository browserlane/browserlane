/// Duplicates a file descriptor.
pub fn dup_fd(fd: i32) -> std::io::Result<i32> {
    // SAFETY: dup on a valid fd.
    let new_fd = unsafe { libc::dup(fd) };
    if new_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(new_fd)
}

/// Resolves once a shutdown signal (SIGINT or SIGTERM) is received.
pub async fn wait_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(_) => return,
    };
    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    }
}
