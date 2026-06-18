use super::process::Browser;
use crate::log;

/// Kills a process and its children on Unix systems by signalling the whole
/// process group, falling back to killing the lone process.
pub(crate) fn kill_process(cmd: &Browser) {
    let mut child = cmd.lock().unwrap();
    let pid = child.id() as libc::pid_t;
    // SAFETY: getpgid/kill are async-signal-safe libc calls on a known pid.
    unsafe {
        let pgid = libc::getpgid(pid);
        if pgid >= 0 {
            libc::kill(-pgid, libc::SIGKILL);
        } else {
            let _ = child.kill();
        }
    }
}

/// Blocks until SIGINT or SIGTERM is delivered.
pub(crate) fn wait_signal() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static SIGNALLED: AtomicBool = AtomicBool::new(false);

    extern "C" fn handler(_sig: libc::c_int) {
        SIGNALLED.store(true, Ordering::SeqCst);
    }

    // SAFETY: installing a trivial async-signal-safe handler.
    unsafe {
        // Cast via an explicit fn pointer (not a direct fn-item->int cast, which
        // newer clippy flags as fn_to_numeric_cast).
        let h = handler as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::signal(libc::SIGINT, h);
        libc::signal(libc::SIGTERM, h);
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
