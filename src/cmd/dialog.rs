use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn dialog_command() -> Command {
    Command::new("dialog")
        .about("Handle browser dialogs (alert, confirm, prompt)")
        .subcommand(
            Command::new("accept")
                .about("Accept a dialog (optionally with prompt text)")
                .arg(Arg::new("text").num_args(0..=1)),
        )
        .subcommand(Command::new("dismiss").about("Dismiss a dialog"))
}

pub async fn run_dialog(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("accept", sub)) => {
            let mut args = Map::new();
            if let Some(text) = sub.get_one::<String>("text") {
                args.insert("text".to_string(), Value::from(text.clone()));
            }
            match daemon_call("browser_dialog_accept", args, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        Some(("dismiss", _)) => {
            match daemon_call("browser_dialog_dismiss", Map::new(), headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        // No subcommand: mirror Go's `cmd.Help()`.
        _ => {
            let _ = dialog_command().print_help();
            println!();
        }
    }
}
