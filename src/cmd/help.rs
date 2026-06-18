//! cobra-faithful `--help` handling. The exact help text is captured from the Go
//! binary into `help_text.rs` (with a sentinel for the program name); here we
//! resolve which command's help to show and substitute the live program name.

use super::diagnostics::prog_name;
use super::help_text::{help_text, COMMAND_PATHS, PROG_SENTINEL};

/// Parent commands whose cobra `Run` is `cmd.Help()` (no default action), so an
/// invocation with no subcommand prints their help. The other parent commands
/// (cookies, find, scroll, storage, wait) have a real default `Run` and are
/// handled by their own `run_*` function, so they are deliberately excluded.
const HELP_ON_NO_SUBCOMMAND: &[&str] = &[
    // `completion` is now a real dispatched command (cmd/completion.rs): it prints
    // its own help when given no/an unknown shell, so it must NOT be intercepted
    // here (that would also swallow `completion <shell>` since `shell` is a
    // positional, not a subcommand).
    "daemon",
    "dialog",
    "diff",
    "download",
    "is",
    "mouse",
    "page",
    "record",
    "record chunk",
    "record group",
];

/// Returns true if `path` is a parent command that prints its help when invoked
/// without a subcommand (cobra's `Run: cmd.Help()`).
pub fn shows_help_on_no_subcommand(path: &str) -> bool {
    HELP_ON_NO_SUBCOMMAND.contains(&path)
}

/// Substitutes the live program name into a captured help/usage string.
fn with_prog(text: &str) -> String {
    text.replace(PROG_SENTINEL, &prog_name())
}

/// Returns whether `path` has child commands (so an unmatched trailing token is
/// an unknown subcommand rather than a positional argument). The root ("") is a
/// parent of everything.
pub fn is_parent(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    let prefix = format!("{path} ");
    COMMAND_PATHS.iter().any(|c| c.starts_with(&prefix))
}

/// Resolves the longest valid command-path prefix from the non-flag tokens.
/// Returns (path, leftover) where `leftover` is true if a non-flag token could
/// not be consumed into the path.
fn resolve_path(args: &[String]) -> (String, bool) {
    let tokens: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| !a.starts_with('-'))
        .collect();
    let mut path = String::new();
    let mut consumed = 0;
    for tok in &tokens {
        let cand = if path.is_empty() {
            (*tok).to_string()
        } else {
            format!("{path} {tok}")
        };
        if COMMAND_PATHS.contains(&cand.as_str()) {
            path = cand;
            consumed += 1;
        } else {
            break;
        }
    }
    (path, consumed < tokens.len())
}

/// If the args request `--help`/`-h` for a valid command, returns its help text
/// (program name substituted) ready to print to stdout. Returns None when there
/// is no help flag, or the path is an unknown subcommand (so clap reports the
/// "unknown command" error like cobra).
pub fn intercept_help(args: &[String]) -> Option<String> {
    if !args.iter().any(|a| a == "--help" || a == "-h") {
        return None;
    }
    let (path, leftover) = resolve_path(args);
    // A leftover token under a parent command is an unknown subcommand: defer to
    // clap so it emits the cobra "unknown command" error instead of help.
    if leftover && is_parent(&path) {
        return None;
    }
    help_text(&path).map(with_prog)
}

/// Returns the root command's help text (program name substituted). Used when no
/// subcommand is given, mirroring cobra's root `Run: cmd.Help()`.
pub fn root_help() -> String {
    with_prog(help_text("").unwrap_or(""))
}

/// Returns a command's full help text (program name substituted), or None.
/// Used when a parent command is invoked without a subcommand (cobra's
/// `Run: cmd.Help()`).
pub fn command_help(path: &str) -> Option<String> {
    help_text(path).map(with_prog)
}

/// Returns the usage portion of a command's help (everything from the `Usage:`
/// line onward, i.e. cobra's `UsageString`), program name substituted, or None.
/// cobra prints this after `Error:` on argument/flag errors.
#[allow(dead_code)]
pub fn usage_string(path: &str) -> Option<String> {
    let text = help_text(path)?;
    let usage = match text.find("Usage:") {
        Some(idx) => &text[idx..],
        None => text,
    };
    Some(with_prog(usage))
}
