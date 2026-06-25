use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn title_command() -> Command {
    Command::new("title")
        .about("Get the current page title")
        .after_help(examples(&[("title", "Prints: Example Domain")]))
}

pub async fn run_title(headless: bool, json_output: bool) {
    match daemon_call("browser_get_title", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
