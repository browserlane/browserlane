use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn keys_command() -> Command {
    Command::new("keys")
        .about("Press a key or key combination")
        .arg(
            Arg::new("keys")
                .required(true)
                .num_args(1)
                .help("Key or key combination to press (e.g., Enter, Control+a, Shift+Tab)"),
        )
        .after_help(examples(&[
            ("keys Enter", "Press Enter"),
            ("keys \"Control+a\"", "Select all"),
            ("keys \"Shift+Tab\"", "Shift+Tab to previous field"),
        ]))
}

pub async fn run_keys(keys: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("keys".to_string(), Value::from(keys));
    match daemon_call("browser_keys", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
