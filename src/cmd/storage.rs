use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{extract_text, print_error, print_result};

pub fn storage_command() -> Command {
    Command::new("storage")
        .about("Export or restore browser state (cookies, localStorage, sessionStorage)")
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file path"),
        )
        .subcommand(
            Command::new("restore")
                .about("Restore browser state from a JSON file")
                .arg(Arg::new("path").required(true).num_args(1)),
        )
}

pub async fn run_storage(matches: &ArgMatches, headless: bool, json_output: bool) {
    if let Some(("restore", sub)) = matches.subcommand() {
        let path_arg = sub.get_one::<String>("path").cloned().unwrap_or_default();
        let path = match std::path::absolute(&path_arg) {
            Ok(p) => p.to_string_lossy().into_owned(),
            Err(e) => {
                eprintln!("Error: invalid path: {e}");
                std::process::exit(1);
            }
        };

        let mut args = Map::new();
        args.insert("path".to_string(), Value::from(path));
        match daemon_call("browser_restore_storage", args, headless).await {
            Ok(result) => print_result(&result, json_output),
            Err(e) => print_error(&e, json_output),
        }
        return;
    }

    let output = matches.get_one::<String>("output").cloned().unwrap_or_default();

    match daemon_call("browser_storage_state", Map::new(), headless).await {
        Ok(result) => {
            if !output.is_empty() {
                let text = extract_text(&result);
                if let Err(e) = std::fs::write(&output, text) {
                    print_error(&anyhow::anyhow!("failed to write file: {e}"), json_output);
                }
                println!("State saved to {output}");
                return;
            }
            print_result(&result, json_output);
        }
        Err(e) => print_error(&e, json_output),
    }
}
