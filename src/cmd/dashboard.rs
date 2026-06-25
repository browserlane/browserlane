//! The compact, branded launch screen shown when `bl` is run with no arguments
//! on an interactive terminal.
//!
//! `main` only calls this when stdout is a TTY and `--json` is not set; for
//! pipes, redirects, and `--json` it serves the grouped root help
//! (`render_root_help`) instead, so scripts get a stable command listing. Colour
//! is applied through `anstream`, which strips it under `NO_COLOR` or on a
//! non-TTY stream.

use std::fmt::Write as _;
use std::io::Write as _;

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

/// The five rows of the `bl` figlet, each padded to a common width so the
/// right-hand text column lines up. Kept as a raw string so the `\` and `|`
/// render literally with no escaping.
const LOGO: &str = r"  _     _
 | |__ | |
 | '_ \| |
 | |_) | |
 |_.__/|_|";

/// Builds the full no-args launch screen as a styled string. Splitting the
/// build from the write lets tests capture the exact layout (stripping ANSI)
/// while the real path still streams through `anstream`.
fn render_dashboard() -> String {
    let p = prog_name();
    let fulls: Vec<String> = EXAMPLES.iter().map(|(c, _)| format!("{p} {c}")).collect();
    let width = fulls.iter().map(String::len).max().unwrap_or(0);

    // Right-hand text for each logo row: the wordmark renders in BRAND with the
    // logo; the three info lines render DIM. Row 0 has no text.
    let version = format!("v{VERSION}");
    let info: [(&str, bool); 5] = [
        ("", false),
        ("browserlane", true),
        ("browser automation for humans and AI agents", false),
        ("one binary · CLI + MCP · over WebDriver BiDi", false),
        (version.as_str(), false),
    ];

    let mut out = String::new();
    let _ = writeln!(out);
    for (art, (text, is_brand)) in LOGO.lines().zip(info.iter()) {
        if text.is_empty() {
            let _ = writeln!(out, "{BRAND}{art}{BRAND:#}");
        } else if *is_brand {
            let _ = writeln!(out, "{BRAND}{art}   {text}{BRAND:#}");
        } else {
            let _ = writeln!(out, "{BRAND}{art}{BRAND:#}   {DIM}{text}{DIM:#}");
        }
    }
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
    out
}

/// Prints the no-args launch screen to stdout (through `anstream`, which strips
/// colour on a non-TTY stream and under `NO_COLOR`).
pub fn print_dashboard() {
    let mut out = anstream::stdout();
    let _ = write!(out, "{}", render_dashboard());
}

#[cfg(test)]
/// The launch screen as plain text (ANSI stripped) for snapshotting the layout.
pub(crate) fn dashboard_plain() -> String {
    anstream::adapter::strip_str(&render_dashboard()).to_string()
}
