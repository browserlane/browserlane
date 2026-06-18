// Package plumbing for clicker/internal/process.
// Mirrors the flat Go `process` package: re-export the shared logic plus the
// platform-gated WithCleanup helper.

mod process;
#[cfg(unix)]
mod process_unix;
#[cfg(windows)]
mod process_windows;

// Consumed by later phases (browser launcher, serve/daemon lifecycle).
#[allow(unused_imports)]
pub use process::*;

#[cfg(unix)]
#[allow(unused_imports)]
pub use process_unix::with_cleanup;
#[cfg(windows)]
#[allow(unused_imports)]
pub use process_windows::with_cleanup;
