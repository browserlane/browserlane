use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};

/// Unix-specific Chrome launch arguments (none).
pub(crate) fn platform_chrome_args() -> Vec<String> {
    Vec::new()
}

/// Starts the command as a process-group leader so the whole group can be
/// killed atomically (Go: `SysProcAttr{Setpgid: true}`).
pub(crate) fn set_proc_group(cmd: &mut Command) {
    cmd.process_group(0);
}

/// Kills all processes in the process group of the given PID.
pub(crate) fn kill_process_group(pid: i32) {
    // SAFETY: getpgid/kill are async-signal-safe libc calls.
    unsafe {
        let pgid = libc::getpgid(pid);
        if pgid >= 0 {
            libc::kill(-pgid, libc::SIGKILL);
        }
    }
}

/// Sends SIGKILL to a process by PID.
pub(crate) fn kill_by_pid(pid: i32) {
    // SAFETY: kill is async-signal-safe.
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
}

/// Polls until the given PID has exited or the timeout is reached.
pub(crate) fn wait_for_process_dead(pid: i32, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        // kill(pid, 0) returns non-zero once the process no longer exists.
        if unsafe { libc::kill(pid, 0) } != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
