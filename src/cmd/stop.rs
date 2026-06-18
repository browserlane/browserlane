use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn stop_command() -> Command {
    Command::new("stop").about("Stop the browser session")
}

pub async fn run_stop(headless: bool, json_output: bool) {
    match daemon_call("browser_stop", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
