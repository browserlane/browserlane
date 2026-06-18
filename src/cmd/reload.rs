use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn reload_command() -> Command {
    Command::new("reload").about("Reload the current page")
}

pub async fn run_reload(headless: bool, json_output: bool) {
    match daemon_call("browser_reload", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
