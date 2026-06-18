use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn uncheck_command() -> Command {
    Command::new("uncheck")
        .about("Uncheck a checkbox")
        .arg(Arg::new("selector").required(true).num_args(1))
}

pub async fn run_uncheck(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    match daemon_call("browser_uncheck", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
