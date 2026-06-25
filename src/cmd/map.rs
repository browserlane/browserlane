use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn map_command() -> Command {
    Command::new("map")
        .about("Map interactive page elements with @refs")
        .arg(
            Arg::new("selector")
                .long("selector")
                .help("Scope to elements within this CSS selector"),
        )
        .after_help(examples(&[
            (
                "map",
                "Lists interactive elements with refs like @e1, @e2\n  # Use refs with other commands: {prog} click @e1",
            ),
            (
                "map --selector \"nav\"",
                "Only map elements inside the <nav> element",
            ),
        ]))
}

pub async fn run_map(matches: &ArgMatches, headless: bool, json_output: bool) {
    let mut args = Map::new();
    if let Some(sel) = matches.get_one::<String>("selector") {
        if !sel.is_empty() {
            args.insert("selector".to_string(), Value::from(sel.clone()));
        }
    }
    match daemon_call("browser_map", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
