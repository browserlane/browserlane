use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn press_command() -> Command {
    Command::new("press")
        .about("Press a key on a specific element or the focused element")
        .arg(Arg::new("args").required(true).num_args(1..=2))
}

pub async fn run_press(args: Vec<String>, headless: bool, json_output: bool) {
    let mut tool_args = Map::new();
    tool_args.insert("key".to_string(), Value::from(args[0].clone()));
    if args.len() == 2 {
        tool_args.insert("selector".to_string(), Value::from(args[1].clone()));
    }

    match daemon_call("browser_press", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
