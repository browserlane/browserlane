use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn navigate_command() -> Command {
    Command::new("go")
        .about("Go to a URL and print page info")
        .arg(Arg::new("url").required(true).num_args(1).help("URL to navigate to"))
}

pub async fn run_navigate(url: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("url".to_string(), Value::from(url));

    match daemon_call("browser_navigate", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
