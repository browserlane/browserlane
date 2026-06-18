use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn page_command() -> Command {
    Command::new("page")
        .about("Manage browser pages (new, close, switch)")
        .subcommand(
            Command::new("new")
                .about("Open a new browser page")
                .arg(Arg::new("url").num_args(0..=1)),
        )
        .subcommand(
            Command::new("close")
                .about("Close a browser page by index (default: current page)")
                .arg(Arg::new("index").num_args(0..=1)),
        )
        .subcommand(
            Command::new("switch")
                .about("Switch to a browser page by index or URL substring")
                .arg(Arg::new("target").required(true).num_args(1)),
        )
}

pub async fn run_page(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("new", sub)) => {
            let mut args = Map::new();
            if let Some(url) = sub.get_one::<String>("url") {
                args.insert("url".to_string(), Value::from(url.clone()));
            }
            call("browser_new_page", args, headless, json_output).await;
        }
        Some(("close", sub)) => {
            let mut args = Map::new();
            if let Some(idx_arg) = sub.get_one::<String>("index") {
                match idx_arg.parse::<i64>() {
                    Ok(idx) => {
                        args.insert("index".to_string(), Value::from(idx));
                    }
                    Err(_) => {
                        eprintln!("Error: invalid page index: {idx_arg}");
                        std::process::exit(1);
                    }
                }
            }
            call("browser_close_page", args, headless, json_output).await;
        }
        Some(("switch", sub)) => {
            let target = sub.get_one::<String>("target").cloned().unwrap_or_default();
            let mut args = Map::new();
            // Try to parse as integer index, else treat as URL substring.
            if let Ok(idx) = target.parse::<i64>() {
                args.insert("index".to_string(), Value::from(idx));
            } else {
                args.insert("url".to_string(), Value::from(target));
            }
            call("browser_switch_page", args, headless, json_output).await;
        }
        _ => {
            // Parent `page` with no subcommand prints help (Go: cmd.Help()).
            let _ = page_command().print_help();
            println!();
        }
    }
}

async fn call(tool: &str, args: Map<String, Value>, headless: bool, json_output: bool) {
    match daemon_call(tool, args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
