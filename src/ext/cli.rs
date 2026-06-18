//! browserlane-specific CLI subcommands.
//!
//! Register new clap subcommands in [`register`] and dispatch them in
//! [`dispatch`]. Free to use any crate utilities (logging, paths, etc.) and
//! depend on `crate::*`; do not modify anything under `src/` other than the
//! seam call sites.

use clap::{Arg, ArgAction, ArgMatches, Command};

/// Register all browserlane-specific subcommands on the root CLI.
pub fn register(cli: Command) -> Command {
    cli.subcommand(
        Command::new("inspect").about("Print browserlane build + identity metadata as JSON"),
    )
    .subcommand(
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
            ),
    )
}

/// Dispatch a browserlane-specific subcommand. Returns `true` if handled.
pub async fn dispatch(name: &str, sub: &ArgMatches, _headless: bool, _json_output: bool) -> bool {
    match name {
        "inspect" => {
            run_inspect();
            true
        }
        "add-mcp" => {
            run_add_mcp(sub);
            true
        }
        _ => false,
    }
}

fn run_inspect() {
    let info = serde_json::json!({
        "name": "browserlane",
        "binary": "bl",
        "version": crate::VERSION,
        "target_os": std::env::consts::OS,
        "target_arch": std::env::consts::ARCH,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&info).unwrap_or_else(|_| info.to_string())
    );
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
