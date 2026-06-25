use super::process::Browser;
use crate::log;

/// Kills a process tree on Windows via `taskkill /T /F /PID`.
pub(crate) fn kill_process(cmd: &Browser) {
    let pid = cmd.lock().unwrap().id();
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .status();
}

/// Blocks until a console interrupt (Ctrl-C / SIGINT) is delivered.
pub(crate) fn wait_signal() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static SIGNALLED: AtomicBool = AtomicBool::new(false);

    extern "C" fn handler(_sig: libc::c_int) {
        SIGNALLED.store(true, Ordering::SeqCst);
    }

    // SAFETY: installing a trivial handler for SIGINT.
    unsafe {
        libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t);
    }

    while !SIGNALLED.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Wraps a closure with panic recovery that ensures browser cleanup, then
/// re-raises the panic (mirrors Go's deferred recover + re-panic).
pub fn with_cleanup<F: FnOnce() + std::panic::UnwindSafe>(f: F) {
    if let Err(payload) = std::panic::catch_unwind(f) {
        log::error("panic recovered, cleaning up browsers");
        super::process::kill_all();
        std::panic::resume_unwind(payload);
    }
}
