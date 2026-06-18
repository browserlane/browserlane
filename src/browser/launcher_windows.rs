use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};

/// Windows-specific Chrome launch arguments. The Chrome for Testing sandbox
/// cannot access its own executable in AppData due to filesystem permissions.
pub(crate) fn platform_chrome_args() -> Vec<String> {
    vec!["--no-sandbox".to_string()]
}

/// Prevents chromedriver from inheriting the parent's console window
/// (CREATE_NO_WINDOW), stopping Chrome's stderr from leaking to the terminal.
pub(crate) fn set_proc_group(cmd: &mut Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// No-op on Windows — `kill_by_pid` uses `taskkill /T` which kills the tree.
pub(crate) fn kill_process_group(_pid: i32) {}

/// Kills a process tree by PID on Windows.
pub(crate) fn kill_by_pid(pid: i32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .status();
}

/// Polls until the given PID has exited or the timeout is reached.
pub(crate) fn wait_for_process_dead(pid: i32, timeout: Duration) {
    std::thread::sleep(Duration::from_millis(50));

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output();
        match out {
            Err(_) => return,
            Ok(o) => {
                if !String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()) {
                    return;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
