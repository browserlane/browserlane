use std::process::Command;

/// Configures the child process to run in a new session (detached).
pub fn set_sys_proc_attr(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid in the child between fork and exec is async-signal-safe.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}
