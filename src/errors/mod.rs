// Package plumbing for clicker/internal/errors.
// Mirrors the flat Go `errors` package by re-exporting the ported file's items.

mod errors;

// Consumed by later phases (api/bidi error paths).
#[allow(unused_imports)]
pub use errors::*;
