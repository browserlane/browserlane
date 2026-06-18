use std::process::Command;

/// Configures the detached daemon child to start in a new process group so it
/// outlives the launching CLI (Go: `SysProcAttr{CreationFlags:
/// CREATE_NEW_PROCESS_GROUP}`).
pub fn set_sys_proc_attr(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}
