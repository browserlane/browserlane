use clap::Command;
use serde_json::Map;

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn pages_command() -> Command {
    Command::new("pages").about("List all open browser pages").after_help(examples(&[(
        "pages",
        "[0] https://example.com\n  # [1] https://google.com",
    )]))
}

pub async fn run_pages(headless: bool, json_output: bool) {
    match daemon_call("browser_list_pages", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
