use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn viewport_command() -> Command {
    Command::new("viewport")
        .about("Get or set the browser viewport size")
        .arg(Arg::new("args").num_args(0..=2))
        .arg(
            Arg::new("dpr")
                .long("dpr")
                .value_parser(clap::value_parser!(f64))
                .help("Device pixel ratio (e.g., 2 for Retina)"),
        )
}

pub async fn run_viewport(args: Vec<String>, dpr: f64, headless: bool, json_output: bool) {
    if args.is_empty() {
        match daemon_call("browser_get_viewport", Map::new(), headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    if args.len() == 1 {
        eprintln!("Error: provide both width and height");
        std::process::exit(1);
    }

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

    let mut call_args = Map::new();
    call_args.insert("width".to_string(), Value::from(width as f64));
    call_args.insert("height".to_string(), Value::from(height as f64));
    if dpr > 0.0 {
        call_args.insert("devicePixelRatio".to_string(), Value::from(dpr));
    }

    match daemon_call("browser_set_viewport", call_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
