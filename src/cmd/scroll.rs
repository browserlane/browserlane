use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn scroll_command() -> Command {
    Command::new("scroll")
        .about("Scroll the page or an element")
        .arg(Arg::new("direction").num_args(0..=1))
        .arg(
            Arg::new("amount")
                .long("amount")
                .default_value("3")
                .value_parser(clap::value_parser!(i64))
                .help("Number of scroll increments"),
        )
        .arg(
            Arg::new("selector")
                .long("selector")
                .default_value("")
                .help("CSS selector for element to scroll to"),
        )
        .subcommand(
            Command::new("into-view")
                .about("Scroll an element into view")
                .arg(Arg::new("selector").required(true).num_args(1)),
        )
}

pub async fn run_scroll(matches: &ArgMatches, headless: bool, json_output: bool) {
    if let Some(("into-view", sub)) = matches.subcommand() {
        let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
        let mut args = Map::new();
        args.insert("selector".to_string(), Value::from(selector));
        match daemon_call("browser_scroll_into_view", args, headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    let direction = matches.get_one::<String>("direction").cloned().unwrap_or_else(|| "down".to_string());
    let amount = *matches.get_one::<i64>("amount").unwrap_or(&3);
    let selector = matches.get_one::<String>("selector").cloned().unwrap_or_default();

    let mut tool_args = Map::new();
    tool_args.insert("direction".to_string(), Value::from(direction));
    tool_args.insert("amount".to_string(), Value::from(amount));
    if !selector.is_empty() {
        tool_args.insert("selector".to_string(), Value::from(selector));
    }

    match daemon_call("browser_scroll", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
