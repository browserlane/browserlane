use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn hover_command() -> Command {
    Command::new("hover")
        .about("Hover over an element by CSS selector")
        .arg(
            Arg::new("args")
                .required(true)
                .num_args(1..=2)
                .help("[url] selector — optional URL to navigate first, then the selector to hover"),
        )
        .after_help(examples(&[
            ("hover \"a\"", "Hover over first link"),
            ("hover https://example.com \"a\"", "Navigate then hover"),
        ]))
}

pub async fn run_hover(args: Vec<String>, headless: bool, json_output: bool) {
    let selector = if args.len() == 2 {
        let mut m = Map::new();
        m.insert("url".to_string(), Value::from(args[0].clone()));
        if let Err(e) = daemon_call("browser_navigate", m, headless).await {
            print_error(&e, json_output);
        }
        args[1].clone()
    } else {
        args[0].clone()
    };

    let mut tool_args = Map::new();
    tool_args.insert("selector".to_string(), Value::from(selector));
    match daemon_call("browser_hover", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
