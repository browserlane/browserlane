use anyhow::anyhow;

use crate::paths;

#[cfg(unix)]
use super::process_unix::process_exists;
#[cfg(windows)]
use super::process_windows::process_exists;

/// Writes the current process PID to the PID file.
pub fn write_pid() -> anyhow::Result<()> {
    let pid_path = paths::get_pid_path().map_err(|e| anyhow!("get PID path: {e}"))?;
    let dir = paths::get_daemon_dir().map_err(|e| anyhow!("get daemon dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| anyhow!("create daemon dir: {e}"))?;
    std::fs::write(&pid_path, std::process::id().to_string())?;
    Ok(())
}

/// Reads the PID from the PID file. Returns 0 if the file doesn't exist.
pub fn read_pid() -> anyhow::Result<i32> {
    let pid_path = paths::get_pid_path()?;
    match std::fs::read_to_string(&pid_path) {
        Ok(data) => data
            .trim()
            .parse::<i32>()
            .map_err(|e| anyhow!("invalid PID file content: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e.into()),
    }
}

/// Removes the PID file.
pub fn remove_pid() -> anyhow::Result<()> {
    let pid_path = paths::get_pid_path()?;
    match std::fs::remove_file(&pid_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Removes PID and socket files if the recorded process is no longer running.
pub fn clean_stale() {
    let pid = match read_pid() {
        Ok(p) => p,
        Err(_) => return,
    };
    if pid == 0 {
        return;
    }
    if process_exists(pid) {
        return;
    }

    let _ = remove_pid();

    // On Unix, remove the stale socket file. On Windows, named pipes are kernel-
    // managed and don't leave files to clean up.
    if !cfg!(windows) {
        if let Ok(socket_path) = paths::get_socket_path() {
            let _ = std::fs::remove_file(socket_path);
        }
    }
}
