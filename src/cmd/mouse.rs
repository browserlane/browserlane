use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn mouse_command() -> Command {
    let button_arg = || {
        Arg::new("button")
            .long("button")
            .default_value("0")
            .value_parser(clap::value_parser!(i64))
            .help("Mouse button (0=left, 1=middle, 2=right)")
    };

    Command::new("mouse")
        .about("Mouse control (click, move, down, up)")
        .subcommand(
            Command::new("click")
                .about("Click at coordinates or current position")
                .arg(Arg::new("coords").num_args(0..=2))
                .arg(button_arg()),
        )
        .subcommand(
            Command::new("move")
                .about("Move the mouse to coordinates")
                .arg(Arg::new("coords").required(true).num_args(2)),
        )
        .subcommand(Command::new("down").about("Press a mouse button down").arg(button_arg()))
        .subcommand(Command::new("up").about("Release a mouse button").arg(button_arg()))
}

pub async fn run_mouse(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("click", sub)) => {
            let button = *sub.get_one::<i64>("button").unwrap_or(&0);
            let coords: Vec<String> = sub
                .get_many::<String>("coords")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            if !coords.is_empty() && coords.len() != 2 {
                eprintln!("Error: accepts 0 or 2 arg(s), received {}", coords.len());
                std::process::exit(1);
            }

            let mut params = Map::new();
            params.insert("button".to_string(), Value::from(button));
            if coords.len() == 2 {
                let x = parse_coord(&coords[0], "x");
                let y = parse_coord(&coords[1], "y");
                params.insert("x".to_string(), Value::from(x));
                params.insert("y".to_string(), Value::from(y));
            }
            match daemon_call("browser_mouse_click", params, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        Some(("move", sub)) => {
            let coords: Vec<String> = sub
                .get_many::<String>("coords")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let x = parse_coord(&coords[0], "x");
            let y = parse_coord(&coords[1], "y");
            let mut params = Map::new();
            params.insert("x".to_string(), Value::from(x));
            params.insert("y".to_string(), Value::from(y));
            match daemon_call("browser_mouse_move", params, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        Some(("down", sub)) => {
            let button = *sub.get_one::<i64>("button").unwrap_or(&0);
            let mut params = Map::new();
            params.insert("button".to_string(), Value::from(button));
            match daemon_call("browser_mouse_down", params, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        Some(("up", sub)) => {
            let button = *sub.get_one::<i64>("button").unwrap_or(&0);
            let mut params = Map::new();
            params.insert("button".to_string(), Value::from(button));
            match daemon_call("browser_mouse_up", params, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        _ => {
            // Parent `mouse` with no subcommand prints help (Go: cmd.Help()).
            let _ = mouse_command().print_help();
            println!();
        }
    }
}

/// Parses a coordinate string to f64, exiting with a Go-style error on failure.
fn parse_coord(s: &str, axis: &str) -> f64 {
    match s.parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: invalid {axis} coordinate: {s}");
            std::process::exit(1);
        }
    }
}
