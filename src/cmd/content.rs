use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::{examples, PROG};
use super::output::{print_error, print_result};

pub fn content_command() -> Command {
    Command::new("content")
        .about("Replace the page HTML content")
        .arg(
            Arg::new("html")
                .num_args(0..=1)
                .help("HTML to set as the page content"),
        )
        .arg(
            Arg::new("stdin")
                .long("stdin")
                .action(clap::ArgAction::SetTrue)
                .help("Read HTML from stdin"),
        )
        .after_help(examples(&[
            ("content \"<h1>Hello World</h1>\"", "Set page content directly"),
            (
                &format!("echo \"<h1>Hello</h1>\" | {PROG} content --stdin"),
                "Set page content from stdin",
            ),
        ]))
}

pub async fn run_content(html_arg: Option<String>, use_stdin: bool, headless: bool, json_output: bool) {
    let html = if use_stdin {
        match std::io::read_to_string(std::io::stdin()) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error reading stdin: {e}");
                std::process::exit(1);
            }
        }
    } else if let Some(html) = html_arg {
        html
    } else {
        eprintln!("Error: html argument or --stdin flag is required");
        std::process::exit(1);
    };

    let mut args = Map::new();
    args.insert("html".to_string(), Value::from(html));
    match daemon_call("browser_set_content", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
