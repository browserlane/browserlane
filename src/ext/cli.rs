//! browserlane-specific CLI subcommands.
//!
//! Register new clap subcommands in [`register`] and dispatch them in
//! [`dispatch`]. Free to use any crate utilities (logging, paths, etc.) and
//! depend on `crate::*`; do not modify anything under `src/` other than the
//! seam call sites.

use clap::{Arg, ArgAction, ArgMatches, Command};

/// Register all browserlane-specific subcommands on the root CLI.
pub fn register(cli: Command) -> Command {
    // These ext-seam commands are clap-native already (no help_text.rs entry).
    // We only add the `Examples:` after_help block, built to match the shared
    // cmd::examples helper's layout. That helper lives in a private module we
    // can't reach from here, so `examples()` below reproduces its format using
    // the re-exported crate::cmd::prog_name().
    cli.subcommand(
        Command::new("add-mcp")
            .about("Register the browserlane MCP server with a coding agent (claude, claude-desktop, cursor, vscode, codex)")
            .arg(
                Arg::new("client")
                    .help("Target client: claude | claude-desktop | cursor | vscode | codex")
                    .required(false),
            )
            .arg(
                Arg::new("list")
                    .long("list")
                    .action(ArgAction::SetTrue)
                    .help("List the supported clients"),
            )
            .arg(
                Arg::new("stdout")
                    .long("stdout")
                    .action(ArgAction::SetTrue)
                    .help("Print the config snippet instead of writing it"),
            )
            .after_help(examples(&[
                ("add-mcp", "List the supported clients"),
                ("add-mcp claude", "Register the MCP server with Claude Code"),
                (
                    "add-mcp cursor --stdout",
                    "Print the Cursor config snippet instead of writing it",
                ),
            ])),
    )
}

/// Builds an `Examples:` after_help block matching the shared `cmd::examples`
/// helper's layout (which is in a private module unreachable from the ext seam).
/// Each `(snippet, comment)` renders as `  <prog> <snippet>` then `  # <comment>`,
/// with the live program name from `crate::cmd::prog_name()`.
fn examples(pairs: &[(&str, &str)]) -> String {
    let prog = crate::cmd::prog_name();
    let mut out = String::from("Examples:");
    for (i, (snippet, comment)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str("\n  ");
        out.push_str(&prog);
        if !snippet.is_empty() {
            out.push(' ');
            out.push_str(snippet);
        }
        if !comment.is_empty() {
            out.push_str("\n  # ");
            out.push_str(comment);
        }
    }
    out
}

/// Dispatch a browserlane-specific subcommand. Returns `true` if handled.
pub async fn dispatch(name: &str, sub: &ArgMatches, _headless: bool, _json_output: bool) -> bool {
    match name {
        "add-mcp" => {
            run_add_mcp(sub);
            true
        }
        _ => false,
    }
}

fn run_add_mcp(sub: &ArgMatches) {
    if sub.get_flag("list") {
        super::add_mcp::list();
        return;
    }
    let stdout = sub.get_flag("stdout");
    match sub.get_one::<String>("client") {
        Some(client) => {
            if let Err(e) = super::add_mcp::add(client, stdout) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        None => super::add_mcp::list(),
    }
}
