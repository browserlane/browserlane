use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn attr_command() -> Command {
    Command::new("attr")
        .about("Get an HTML attribute value from an element")
        .arg(Arg::new("selector").required(true).num_args(1))
        .arg(Arg::new("attribute").required(true).num_args(1))
}

pub async fn run_attr(selector: String, attribute: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    args.insert("attribute".to_string(), Value::from(attribute));

    match daemon_call("browser_get_attribute", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
