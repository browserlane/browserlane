// Package plumbing for clicker/internal/paths.
// Mirrors the flat Go `paths` package by re-exporting the ported file's items.

mod paths;

// Consumed by later phases (browser/daemon path resolution).
#[allow(unused_imports)]
pub use paths::*;
