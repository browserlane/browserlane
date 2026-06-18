//! Structured logging for the clicker library.
//!
//! Mirrors Go's `slog`-based setup: quiet by default (no output), verbose emits
//! JSON diagnostics to stderr. stderr is diagnostics-only per the wire contract,
//! so the exact JSON shape is not part of the parity oracle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// Logging level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// No logging (default).
    Quiet,
    /// All logging (--verbose).
    Verbose,
}

static VERBOSE: AtomicBool = AtomicBool::new(false);
static INIT: Once = Once::new();

/// Configures the global logger with the specified level.
///
/// The underlying subscriber is installed once; subsequent calls only toggle
/// whether records are emitted, matching Go's quiet/verbose switch.
pub fn setup(level: Level) {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .json()
            .with_writer(std::io::stderr)
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    });
    VERBOSE.store(level == Level::Verbose, Ordering::SeqCst);
}

#[inline]
fn enabled() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Logs at debug level.
pub fn debug(msg: &str) {
    if enabled() {
        tracing::debug!("{}", msg);
    }
}

/// Logs at info level.
pub fn info(msg: &str) {
    if enabled() {
        tracing::info!("{}", msg);
    }
}

/// Logs at warn level.
pub fn warn(msg: &str) {
    if enabled() {
        tracing::warn!("{}", msg);
    }
}

/// Logs at error level.
pub fn error(msg: &str) {
    if enabled() {
        tracing::error!("{}", msg);
    }
}
