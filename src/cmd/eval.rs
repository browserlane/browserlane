use std::io::Read;

use anyhow::anyhow;
use clap::{Arg, ArgAction, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn eval_command() -> Command {
    Command::new("eval")
        .about("Evaluate a JavaScript expression (optionally navigate to URL first)")
        .arg(
            Arg::new("args")
                .num_args(0..=2)
                .help("[url] expression — JS to evaluate, optionally preceded by a URL to navigate first"),
        )
        .arg(
            Arg::new("stdin")
                .long("stdin")
                .action(ArgAction::SetTrue)
                .help("Read expression from stdin"),
        )
        .after_help(examples(&[
            ("eval \"document.title\"", "Evaluates on current page"),
            (
                "eval https://example.com \"document.title\"",
                "Navigates to URL first, then evaluates",
            ),
            (
                "eval --stdin",
                "Read expression from stdin, e.g. echo 'document.title' | ... (avoids shell quoting issues)",
            ),
        ]))
}

pub async fn run_eval(args: Vec<String>, use_stdin: bool, headless: bool, json_output: bool) {
    let expression: String = if use_stdin {
        let mut data = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut data) {
            print_error(&anyhow!("failed to read stdin: {e}"), json_output);
        }
        data.trim().to_string()
    } else if args.len() == 2 {
        // eval <url> <expression> — navigate first.
        let mut m = Map::new();
        m.insert("url".to_string(), Value::from(args[0].clone()));
        if let Err(e) = daemon_call("browser_navigate", m, headless).await {
            print_error(&e, json_output);
        }
        args[1].clone()
    } else if args.len() == 1 {
        // eval <expression> — current page.
        args[0].clone()
    } else {
        eprintln!("Error: expression is required (use args or --stdin)");
        std::process::exit(1);
    };

    let mut tool_args = Map::new();
    tool_args.insert("expression".to_string(), Value::from(expression));
    match daemon_call("browser_evaluate", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
