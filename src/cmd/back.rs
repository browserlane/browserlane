use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn back_command() -> Command {
    Command::new("back")
        .about("Navigate back in browser history")
        .after_help(examples(&[(
            "back",
            "Go back one page (like clicking the back button)",
        )]))
}

pub async fn run_back(headless: bool, json_output: bool) {
    match daemon_call("browser_back", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
