//! The compact, branded launch screen shown when `bl` is run with no arguments
//! on an interactive terminal.
//!
//! `main` only calls this when stdout is a TTY and `--json` is not set; for
//! pipes, redirects, and `--json` it serves the plain `root_help()` instead, so
//! scripts and the smoke harness see unchanged bytes. Colour is applied through
//! `anstream`, which strips it under `NO_COLOR` or on a non-TTY stream.

use std::io::Write;

use super::diagnostics::prog_name;
use super::style::{BRAND, COMMAND, DIM, HEADING};
use crate::VERSION;

/// A few high-value commands to greet a first-time user with.
const EXAMPLES: &[(&str, &str)] = &[
    ("install", "download Chrome for Testing (run once)"),
    ("go https://example.com", "open a URL"),
    ("screenshot -o page.png", "capture the current page"),
    ("find role button", "find elements semantically"),
    ("add-mcp claude", "connect the MCP server to your agent"),
];

/// Prints the no-args launch screen to stdout.
pub fn print_dashboard() {
    let p = prog_name();
    let fulls: Vec<String> = EXAMPLES.iter().map(|(c, _)| format!("{p} {c}")).collect();
    let width = fulls.iter().map(String::len).max().unwrap_or(0);

    let mut out = anstream::stdout();
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {BRAND}{p}{BRAND:#} {DIM}v{VERSION}{DIM:#}  ·  browser automation for AI agents and humans"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  {HEADING}Get started{HEADING:#}");
    for (full, (_, desc)) in fulls.iter().zip(EXAMPLES.iter()) {
        let pad = " ".repeat(width.saturating_sub(full.len()));
        let _ = writeln!(out, "    {COMMAND}{full}{COMMAND:#}{pad}   {DIM}{desc}{DIM:#}");
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {DIM}Run{DIM:#} {COMMAND}{p} --help{COMMAND:#} {DIM}to see every command, or{DIM:#} {COMMAND}{p} <command> --help{COMMAND:#} {DIM}for details.{DIM:#}"
    );
    let _ = writeln!(out);
}
