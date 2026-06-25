use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn cookies_command() -> Command {
    Command::new("cookies")
        .about("Manage browser cookies")
        .arg(
            Arg::new("args")
                .num_args(0..=2)
                .help("Cookie name and value to set (omit both to list all cookies)"),
        )
        .after_help(examples(&[
            ("cookies", "List all cookies"),
            ("cookies \"session\" \"abc123\"", "Set a cookie with name and value"),
        ]))
        .subcommand(
            Command::new("clear")
                .about("Clear all cookies")
                .after_help(examples(&[("cookies clear", "Delete all cookies")])),
        )
}

pub async fn run_cookies(matches: &ArgMatches, headless: bool, json_output: bool) {
    if let Some(("clear", _)) = matches.subcommand() {
        match daemon_call("browser_delete_cookies", Map::new(), headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    let args: Vec<String> = matches
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    // cobra: `if len(args) == 2 { set cookie }`. A single arg falls through to
    // the cookie listing, matching the Go binary's behavior.
    if args.len() == 2 {
        let mut tool_args = Map::new();
        tool_args.insert("name".to_string(), Value::from(args[0].clone()));
        tool_args.insert("value".to_string(), Value::from(args[1].clone()));
        match daemon_call("browser_set_cookie", tool_args, headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    match daemon_call("browser_get_cookies", Map::new(), headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
