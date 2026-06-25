use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn value_command() -> Command {
    Command::new("value")
        .about("Get the current value of a form element")
        .arg(
            Arg::new("selector")
                .required(true)
                .num_args(1)
                .help("CSS selector for the form element"),
        )
        .after_help(examples(&[(
            "value \"input[name=email]\"",
            "Print the current value of the email input",
        )]))
}

pub async fn run_value(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));

    match daemon_call("browser_get_value", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
