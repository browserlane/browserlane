use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn sleep_command() -> Command {
    Command::new("sleep")
        .about("Pause execution for a number of milliseconds")
        .arg(Arg::new("ms").required(true).num_args(1))
}

pub async fn run_sleep(ms_arg: String, headless: bool, json_output: bool) {
    let ms: f64 = match ms_arg.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: invalid milliseconds value: {ms_arg}");
            std::process::exit(1);
        }
    };

    let mut args = Map::new();
    args.insert("ms".to_string(), Value::from(ms));
    match daemon_call("browser_sleep", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
