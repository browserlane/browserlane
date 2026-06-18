use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn forward_command() -> Command {
    Command::new("forward").about("Navigate forward in browser history")
}

pub async fn run_forward(headless: bool, json_output: bool) {
    match daemon_call("browser_forward", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
