use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn check_command() -> Command {
    Command::new("check")
        .about("Check a checkbox or radio button")
        .arg(
            Arg::new("selector")
                .required(true)
                .num_args(1)
                .help("CSS selector (or map ref) for the checkbox or radio button"),
        )
        .after_help(examples(&[(
            "check \"input[name=agree]\"",
            "Check the \"agree\" checkbox (idempotent)",
        )]))
}

pub async fn run_check(selector: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    match daemon_call("browser_check", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
