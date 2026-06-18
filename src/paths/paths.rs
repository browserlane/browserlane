//! Platform-specific path resolution for browserlane.
//!
//! Logic is replicated by hand (rather than via the `directories` crate) to
//! guarantee the resolved paths are byte-identical to the Go binary's.

use std::env;
use std::io;
use std::path::PathBuf;

/// Returns the user's home directory, mirroring Go's `os.UserHomeDir`:
/// `%USERPROFILE%` on Windows, `$HOME` elsewhere.
pub(crate) fn user_home_dir() -> io::Result<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    match env::var_os(var) {
        Some(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "$HOME is not defined",
        )),
    }
}

fn env_nonempty(key: &str) -> Option<PathBuf> {
    match env::var_os(key) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => None,
    }
}

/// Returns the platform-specific cache directory for browserlane.
/// Override with `BROWSERLANE_CACHE_DIR`.
/// Linux: `~/.cache/browserlane/`, macOS: `~/Library/Caches/browserlane/`,
/// Windows: `%LOCALAPPDATA%\browserlane\`.
pub fn get_cache_dir() -> io::Result<PathBuf> {
    if let Some(dir) = env_nonempty("BROWSERLANE_CACHE_DIR") {
        return Ok(dir);
    }

    let base_dir: PathBuf = if cfg!(target_os = "linux") {
        match env_nonempty("XDG_CACHE_HOME") {
            Some(xdg) => xdg,
            None => user_home_dir()?.join(".cache"),
        }
    } else if cfg!(target_os = "macos") {
        user_home_dir()?.join("Library").join("Caches")
    } else if cfg!(target_os = "windows") {
        match env_nonempty("LOCALAPPDATA") {
            Some(local) => local,
            None => user_home_dir()?.join("AppData").join("Local"),
        }
    } else {
        user_home_dir()?.join(".cache")
    };

    Ok(base_dir.join("browserlane"))
}

/// Returns the directory where Chrome for Testing is cached.
pub fn get_chrome_for_testing_dir() -> io::Result<PathBuf> {
    Ok(get_cache_dir()?.join("chrome-for-testing"))
}

/// Reads a directory's immediate entries sorted by file name, mirroring Go's
/// `os.ReadDir` (which returns entries sorted by filename). This ordering is
/// load-bearing: when multiple Chrome versions are cached, both binaries must
/// pick the same one.
fn read_dir_sorted(dir: &std::path::Path) -> io::Result<Vec<std::fs::DirEntry>> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// Returns the path to the Chrome for Testing executable.
/// Only checks the browserlane cache — does not fall back to system Chrome.
pub fn get_chrome_executable() -> io::Result<PathBuf> {
    let cft_dir = get_chrome_for_testing_dir()?;

    for entry in read_dir_sorted(&cft_dir)? {
        if entry.file_type()?.is_dir() {
            let chrome_path = chrome_path_in_version(&entry.path());
            if chrome_path.exists() {
                return Ok(chrome_path);
            }
        }
    }

    Err(io::Error::from(io::ErrorKind::NotFound))
}

/// Returns the path to the cached chromedriver.
pub fn get_chromedriver_path() -> io::Result<PathBuf> {
    let cft_dir = get_chrome_for_testing_dir()?;

    for entry in read_dir_sorted(&cft_dir)? {
        if entry.file_type()?.is_dir() {
            let driver_path = chromedriver_path_in_version(&entry.path());
            if driver_path.exists() {
                return Ok(driver_path);
            }
        }
    }

    Err(io::Error::from(io::ErrorKind::NotFound))
}

/// Returns the Chrome executable path within a version directory.
fn chrome_path_in_version(version_dir: &std::path::Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        version_dir
            .join("Google Chrome for Testing.app")
            .join("Contents")
            .join("MacOS")
            .join("Google Chrome for Testing")
    } else if cfg!(target_os = "windows") {
        version_dir.join("chrome.exe")
    } else {
        version_dir.join("chrome")
    }
}

/// Returns the chromedriver path within a version directory.
fn chromedriver_path_in_version(version_dir: &std::path::Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        version_dir.join("chromedriver.exe")
    } else {
        version_dir.join("chromedriver")
    }
}

/// Returns the platform string used by Chrome for Testing.
fn platform_string() -> String {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-arm64".to_string()
        } else {
            "mac-x64".to_string()
        }
    } else if cfg!(target_os = "windows") {
        "win64".to_string()
    } else {
        "linux64".to_string()
    }
}

/// Exported for use by the installer.
pub fn get_platform_string() -> String {
    platform_string()
}

/// Returns the directory for daemon files (socket, PID).
/// Reuses the cache directory since daemon files are ephemeral.
pub fn get_daemon_dir() -> io::Result<PathBuf> {
    get_cache_dir()
}

/// Returns the platform-specific socket path for the daemon.
/// macOS/Linux: `<cache>/browserlane.sock`; Windows: `\\.\pipe\browserlane`.
pub fn get_socket_path() -> io::Result<PathBuf> {
    if cfg!(target_os = "windows") {
        return Ok(PathBuf::from(r"\\.\pipe\browserlane"));
    }
    Ok(get_daemon_dir()?.join("browserlane.sock"))
}

/// Returns the path to the daemon PID file.
pub fn get_pid_path() -> io::Result<PathBuf> {
    Ok(get_daemon_dir()?.join("browserlane.pid"))
}

/// Returns the platform-specific default directory for screenshots
/// (`~/Pictures/browserlane/` on all platforms).
pub fn get_screenshot_dir() -> io::Result<PathBuf> {
    let home = user_home_dir()?;
    Ok(home.join("Pictures").join("browserlane"))
}
