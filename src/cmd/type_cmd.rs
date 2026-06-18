use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::daemon_cmd::parse_duration_flag;
use super::output::{print_error, print_result};

pub fn type_command() -> Command {
    Command::new("type")
        .about("Type text into an element (optionally navigate to URL first)")
        .arg(Arg::new("args").required(true).num_args(2..=3))
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .default_value("30s")
                .help("Timeout for actionability checks (e.g., 5s, 30s)"),
        )
}

pub async fn run_type(args: Vec<String>, timeout: String, headless: bool, json_output: bool) {
    // Go validates --timeout as a cobra Duration flag at parse time, before Run.
    // Mirror that here so an invalid value errors out before navigating/typing.
    let timeout_ms = match parse_duration_flag("timeout", &timeout) {
        Ok(d) => d.as_millis() as i64,
        Err(msg) => {
            // TODO(P7-CLI): route through the central cobra usage renderer.
            eprintln!("Error: {msg}");
            std::process::exit(1);
        }
    };

    let (selector, text) = if args.len() == 3 {
        // type <url> <selector> <text> — navigate first.
        let mut m = Map::new();
        m.insert("url".to_string(), Value::from(args[0].clone()));
        if let Err(e) = daemon_call("browser_navigate", m, headless).await {
            print_error(&e, json_output);
        }
        (args[1].clone(), args[2].clone())
    } else {
        // type <selector> <text> — current page.
        (args[0].clone(), args[1].clone())
    };

    let mut tool_args = Map::new();
    tool_args.insert("selector".to_string(), Value::from(selector));
    tool_args.insert("text".to_string(), Value::from(text));
    tool_args.insert("timeout".to_string(), Value::from(timeout_ms));

    match daemon_call("browser_type", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
