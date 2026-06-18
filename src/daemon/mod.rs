// Package plumbing for clicker/internal/daemon (persistent session + IPC).
// Mirrors the flat Go `daemon` package. The IPC transport is the `interprocess`
// crate's local socket — a Unix-domain socket on unix, a named pipe on Windows —
// so the daemon/client/router code is platform-agnostic over `Conn`.

mod client;
mod daemon;
#[cfg(unix)]
mod dial_unix;
#[cfg(windows)]
mod dial_windows;
#[cfg(unix)]
mod listener_unix;
#[cfg(windows)]
mod listener_windows;
mod pidfile;
#[cfg(unix)]
mod process_unix;
#[cfg(windows)]
mod process_windows;
mod router;
mod status;

#[cfg(unix)]
use dial_unix::dial;
#[cfg(windows)]
use dial_windows::dial;

/// Cross-platform daemon IPC stream: a Unix-domain socket on unix, a named pipe
/// on Windows (both via the `interprocess` local-socket abstraction).
pub(crate) type Conn = interprocess::local_socket::tokio::Stream;

#[allow(unused_imports)]
pub use client::{call, shutdown, status};
#[allow(unused_imports)]
pub use daemon::{new, Daemon, Options};
#[allow(unused_imports)]
pub use pidfile::{clean_stale, read_pid, remove_pid, write_pid};
#[allow(unused_imports)]
pub use router::StatusResult;
#[allow(unused_imports)]
pub use status::is_running;

#[cfg(unix)]
#[allow(unused_imports)]
pub use process_unix::process_exists;
#[cfg(windows)]
#[allow(unused_imports)]
pub use process_windows::process_exists;
