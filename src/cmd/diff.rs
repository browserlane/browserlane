use clap::{ArgMatches, Command};
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::examples::{examples, PROG};
use super::output::{print_error, print_result};

pub fn diff_command() -> Command {
    Command::new("diff")
        .about("Compare current state vs previous")
        // No subcommand prints this parent's help natively (cobra's `cmd.Help()`).
        .arg_required_else_help(true)
        .subcommand(
            Command::new("map")
                .about("Compare current page elements vs last map")
                .after_help(examples(&[(
                    &format!(
                        "map           # take initial snapshot\n  {PROG} click @e3     # interact with page\n  {PROG} diff map      # see what changed"
                    ),
                    "",
                )])),
        )
}

pub async fn run_diff(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("map", _)) => match daemon_call("browser_diff_map", Map::new(), headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        },
        // No subcommand: mirror Go's `cmd.Help()`.
        _ => {
            let _ = diff_command().print_help();
            println!();
        }
    }
}
