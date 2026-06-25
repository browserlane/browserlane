use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn window_command() -> Command {
    Command::new("window")
        .about("Get or set the OS browser window size, position, or state")
        .arg(
            Arg::new("args")
                .num_args(0..=4)
                .help("Width and height, optionally followed by x and y position (omit all to print the current window)"),
        )
        .arg(
            Arg::new("state")
                .long("state")
                .help("Window state: normal, maximized, minimized, fullscreen"),
        )
        .after_help(examples(&[
            ("window", "{\"state\":\"normal\",\"x\":0,\"y\":25,\"width\":1280,\"height\":720}"),
            ("window 1920 1080", "Set window to 1920x1080"),
            ("window 1920 1080 0 0", "Set window to 1920x1080 at position (0, 0)"),
            ("window --state maximized", "Maximize the window"),
        ]))
}

pub async fn run_window(args: Vec<String>, state: Option<String>, headless: bool, json_output: bool) {
    let state = state.unwrap_or_default();

    if args.is_empty() && state.is_empty() {
        match daemon_call("browser_get_window", Map::new(), headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    if args.len() == 1 || args.len() == 3 {
        eprintln!("Error: provide both width and height");
        std::process::exit(1);
    }

    let mut call_args = Map::new();

    if args.len() >= 2 {
        let width: i64 = match args[0].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid width: {}", args[0]);
                std::process::exit(1);
            }
        };
        let height: i64 = match args[1].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid height: {}", args[1]);
                std::process::exit(1);
            }
        };
        call_args.insert("width".to_string(), Value::from(width as f64));
        call_args.insert("height".to_string(), Value::from(height as f64));
    }

    if args.len() == 4 {
        let x: i64 = match args[2].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid x: {}", args[2]);
                std::process::exit(1);
            }
        };
        let y: i64 = match args[3].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid y: {}", args[3]);
                std::process::exit(1);
            }
        };
        call_args.insert("x".to_string(), Value::from(x as f64));
        call_args.insert("y".to_string(), Value::from(y as f64));
    }

    if !state.is_empty() {
        call_args.insert("state".to_string(), Value::from(state));
    }

    match daemon_call("browser_set_window", call_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
