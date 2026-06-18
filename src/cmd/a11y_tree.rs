use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn a11y_tree_command() -> Command {
    Command::new("a11y-tree")
        .about("Get the accessibility tree of the current page")
        .arg(
            Arg::new("everything")
                .long("everything")
                .action(ArgAction::SetTrue)
                .help("Show all nodes including generic containers"),
        )
}

pub async fn run_a11y_tree(matches: &ArgMatches, headless: bool, json_output: bool) {
    let mut args = Map::new();
    if matches.get_flag("everything") {
        args.insert("everything".to_string(), Value::Bool(true));
    }
    match daemon_call("browser_a11y_tree", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
