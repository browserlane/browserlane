use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn upload_command() -> Command {
    Command::new("upload")
        .about("Set files on an input[type=file] element")
        .arg(Arg::new("selector").required(true))
        .arg(
            Arg::new("files")
                .required(true)
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

pub async fn run_upload(matches: &ArgMatches, headless: bool, json_output: bool) {
    let selector = matches.get_one::<String>("selector").cloned().unwrap_or_default();
    let file_paths: Vec<String> = matches
        .get_many::<String>("files")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    // Resolve to absolute paths.
    let mut abs_files = Vec::with_capacity(file_paths.len());
    for f in &file_paths {
        match std::path::absolute(f) {
            Ok(p) => abs_files.push(Value::from(p.to_string_lossy().to_string())),
            Err(e) => {
                eprintln!("Error: invalid file path {f:?}: {e}");
                std::process::exit(1);
            }
        }
    }

    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    args.insert("files".to_string(), Value::Array(abs_files));
    match daemon_call("browser_upload", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
