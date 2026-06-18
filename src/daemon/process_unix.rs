/// Checks if a process with the given PID exists.
pub fn process_exists(pid: i32) -> bool {
    // SAFETY: kill with signal 0 only tests for process existence.
    let r = unsafe { libc::kill(pid, 0) };
    if r == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
