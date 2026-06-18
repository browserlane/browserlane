/// Checks if a process with the given PID exists. On Windows we rely on the PID
/// file being present (mirrors Go's FindProcess-always-succeeds behavior).
pub fn process_exists(_pid: i32) -> bool {
    true
}
