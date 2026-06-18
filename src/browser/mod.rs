// Package plumbing for clicker/internal/browser (Chrome installer + launcher).
// Mirrors the flat Go `browser` package by re-exporting each ported file's items.

mod installer;
mod launcher;
#[cfg(unix)]
mod launcher_unix;
#[cfg(windows)]
mod launcher_windows;

#[allow(unused_imports)]
pub use installer::*;
#[allow(unused_imports)]
pub use launcher::*;
