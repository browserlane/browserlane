//! Restrained, TTY-aware colour for human-facing CLI output.
//!
//! Everything styled with these is written through an `anstream` stream, which
//! strips ANSI automatically when stdout/stderr is not a terminal and honours
//! `NO_COLOR` / `CLICOLOR` — so piped, redirected, and `NO_COLOR` output is
//! byte-for-byte plain. Never used for `--json`, MCP, or completion output.

use anstyle::{AnsiColor, Color, Style};

/// Brand mark (the program name on the launch screen): bold cyan.
pub const BRAND: Style = Style::new()
    .bold()
    .fg_color(Some(Color::Ansi(AnsiColor::Cyan)));

/// Section heading (e.g. "Get started"): bold.
pub const HEADING: Style = Style::new().bold();

/// A command the user can type, shown in examples: cyan.
pub const COMMAND: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));

/// De-emphasised text — descriptions, hints, the version string: dimmed.
pub const DIM: Style = Style::new().dimmed();

/// The "Error:" prefix on a failed command: bold red.
pub const ERROR: Style = Style::new()
    .bold()
    .fg_color(Some(Color::Ansi(AnsiColor::Red)));
