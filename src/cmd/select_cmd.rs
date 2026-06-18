use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn select_command() -> Command {
    Command::new("select")
        .about("Select an option in a <select> element")
        .arg(Arg::new("selector").required(true).num_args(1))
        .arg(Arg::new("value").required(true).num_args(1))
}

pub async fn run_select(selector: String, value: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    args.insert("value".to_string(), Value::from(value));
    match daemon_call("browser_select", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
