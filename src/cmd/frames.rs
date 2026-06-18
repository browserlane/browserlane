use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn frames_command() -> Command {
    Command::new("frames").about("List all child frames (iframes) on the page")
}

pub async fn run_frames(headless: bool, json_output: bool) {
    match daemon_call("browser_frames", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
