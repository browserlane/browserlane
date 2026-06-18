use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn count_command() -> Command {
    Command::new("count")
        .about("Count matching elements")
        .arg(Arg::new("selector").required(true).num_args(1))
}

pub async fn run_count(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));

    match daemon_call("browser_count", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
