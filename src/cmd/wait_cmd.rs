use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn wait_command() -> Command {
    let timeout_int = || {
        Arg::new("timeout")
            .long("timeout")
            .default_value("30000")
            .value_parser(clap::value_parser!(i64))
            .help("Timeout in milliseconds")
    };
    // Go declares --timeout as Float64 for `wait text` and `wait fn` (wait_cmd.go:98,152)
    // but Int for the parent / url / load. Mirror that so fractional/scientific values
    // (e.g. --timeout 2.7) parse and run as Go does, instead of being rejected by clap.
    let timeout_float = || {
        Arg::new("timeout")
            .long("timeout")
            .default_value("30000")
            .value_parser(clap::value_parser!(f64))
            .help("Timeout in milliseconds")
    };

    Command::new("wait")
        .about("Wait for an element, URL, text, page load, or JS condition")
        .arg(Arg::new("selector").num_args(0..=1))
        .arg(
            Arg::new("state")
                .long("state")
                .default_value("attached")
                .help("State to wait for: attached, visible, hidden"),
        )
        .arg(timeout_int())
        .subcommand(
            Command::new("url")
                .about("Wait until the page URL contains a substring")
                .arg(Arg::new("pattern").required(true).num_args(1))
                .arg(timeout_int()),
        )
        .subcommand(
            Command::new("text")
                .about("Wait until text appears on the page")
                .arg(Arg::new("text").required(true).num_args(1))
                .arg(timeout_float()),
        )
        .subcommand(
            Command::new("load")
                .about("Wait until the page is fully loaded")
                .arg(timeout_int()),
        )
        .subcommand(
            Command::new("fn")
                .about("Wait until a JS expression returns truthy")
                .arg(Arg::new("expression").required(true).num_args(1))
                .arg(timeout_float()),
        )
}

pub async fn run_wait(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("url", sub)) => {
            let pattern = sub.get_one::<String>("pattern").cloned().unwrap_or_default();
            let timeout = *sub.get_one::<i64>("timeout").unwrap_or(&30000);
            let mut args = Map::new();
            args.insert("pattern".to_string(), Value::from(pattern));
            args.insert("timeout".to_string(), Value::from(timeout));
            call("browser_wait_for_url", args, headless, json_output).await;
        }
        Some(("text", sub)) => {
            let text = sub.get_one::<String>("text").cloned().unwrap_or_default();
            let timeout = *sub.get_one::<f64>("timeout").unwrap_or(&30000.0);
            let mut args = Map::new();
            args.insert("text".to_string(), Value::from(text));
            // Go (`wait text`) only forwards the timeout when it is > 0, so a value
            // of 0 falls back to the handler's 30s default instead of waiting 0ms.
            if timeout > 0.0 {
                args.insert("timeout".to_string(), Value::from(timeout));
            }
            call("browser_wait_for_text", args, headless, json_output).await;
        }
        Some(("load", sub)) => {
            let timeout = *sub.get_one::<i64>("timeout").unwrap_or(&30000);
            let mut args = Map::new();
            args.insert("timeout".to_string(), Value::from(timeout));
            call("browser_wait_for_load", args, headless, json_output).await;
        }
        Some(("fn", sub)) => {
            let expression = sub.get_one::<String>("expression").cloned().unwrap_or_default();
            let timeout = *sub.get_one::<f64>("timeout").unwrap_or(&30000.0);
            let mut args = Map::new();
            args.insert("expression".to_string(), Value::from(expression));
            // Go (`wait fn`) only forwards the timeout when it is > 0, so a value
            // of 0 falls back to the handler's 30s default instead of waiting 0ms.
            if timeout > 0.0 {
                args.insert("timeout".to_string(), Value::from(timeout));
            }
            call("browser_wait_for_fn", args, headless, json_output).await;
        }
        _ => {
            // Parent: wait [selector] --state --timeout.
            let selector = match matches.get_one::<String>("selector") {
                Some(s) => s.clone(),
                None => {
                    eprintln!("Error: accepts 1 arg(s), received 0");
                    std::process::exit(1);
                }
            };
            let state = matches.get_one::<String>("state").cloned().unwrap_or_else(|| "attached".to_string());
            let timeout = *matches.get_one::<i64>("timeout").unwrap_or(&30000);
            let mut args = Map::new();
            args.insert("selector".to_string(), Value::from(selector));
            args.insert("state".to_string(), Value::from(state));
            args.insert("timeout".to_string(), Value::from(timeout));
            call("browser_wait", args, headless, json_output).await;
        }
    }
}

async fn call(tool: &str, args: Map<String, Value>, headless: bool, json_output: bool) {
    match daemon_call(tool, args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
