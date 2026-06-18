//! Tracks and manages spawned browser processes.

use std::process::Child;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(unix)]
use super::process_unix::{kill_process, wait_signal};
#[cfg(windows)]
use super::process_windows::{kill_process, wait_signal};

/// A tracked browser process. Shared so the process can be killed from one task
/// while another waits on it (mirrors Go's shared `*exec.Cmd`).
pub type Browser = Arc<Mutex<Child>>;

/// Tracks spawned browser processes.
#[derive(Default)]
pub struct Manager {
    browsers: Mutex<Vec<Browser>>,
}

fn default_manager() -> &'static Manager {
    static MANAGER: OnceLock<Manager> = OnceLock::new();
    MANAGER.get_or_init(Manager::default)
}

/// Adds a browser process to be tracked.
pub fn track(cmd: &Browser) {
    default_manager()
        .browsers
        .lock()
        .unwrap()
        .push(Arc::clone(cmd));
}

/// Removes a browser process from tracking.
pub fn untrack(cmd: &Browser) {
    let mut browsers = default_manager().browsers.lock().unwrap();
    if let Some(idx) = browsers.iter().position(|c| Arc::ptr_eq(c, cmd)) {
        browsers.remove(idx);
    }
}

/// Terminates all tracked browser processes and their children.
pub fn kill_all() {
    let mut browsers = default_manager().browsers.lock().unwrap();
    for cmd in browsers.iter() {
        kill_process(cmd);
    }
    browsers.clear();
}

/// Terminates a specific browser process.
pub fn kill_browser(cmd: &Browser) -> std::io::Result<()> {
    untrack(cmd);
    cmd.lock().unwrap().kill()
}

/// Blocks until SIGINT/SIGTERM is received, then cleans up.
pub fn wait_for_signal() {
    wait_signal();
    kill_all();
}
