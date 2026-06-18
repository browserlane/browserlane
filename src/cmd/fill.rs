use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn fill_command() -> Command {
    Command::new("fill")
        .about("Clear an input field and type new text")
        .arg(Arg::new("selector").required(true).num_args(1))
        .arg(Arg::new("text").required(true).num_args(1))
}

pub async fn run_fill(selector: String, text: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    args.insert("value".to_string(), Value::from(text));

    match daemon_call("browser_fill", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
