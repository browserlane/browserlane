use clap::{ArgMatches, Command};
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn diff_command() -> Command {
    Command::new("diff")
        .about("Compare current state vs previous")
        .subcommand(Command::new("map").about("Compare current page elements vs last map"))
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
