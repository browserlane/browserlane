use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn highlight_command() -> Command {
    Command::new("highlight")
        .about("Highlight an element with a red outline for 3 seconds")
        .arg(
            Arg::new("selector")
                .required(true)
                .num_args(1)
                .help("CSS selector or @-reference of the element to highlight"),
        )
        .after_help(examples(&[
            ("highlight \"h1\"", "Highlights the first h1 element"),
            ("highlight @e1", "Highlights the element from map"),
        ]))
}

pub async fn run_highlight(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    match daemon_call("browser_highlight", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
