use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn focus_command() -> Command {
    Command::new("focus")
        .about("Focus an element")
        .arg(
            Arg::new("selector")
                .required(true)
                .num_args(1)
                .help("CSS selector (or map ref) for the element to focus"),
        )
        .after_help(examples(&[
            ("focus \"input[name=email]\"", "Focus the email input"),
            ("focus @e1", "Focus element from map"),
        ]))
}

pub async fn run_focus(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    match daemon_call("browser_focus", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
