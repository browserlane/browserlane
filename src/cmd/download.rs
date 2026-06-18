use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn download_command() -> Command {
    Command::new("download")
        .about("Manage browser downloads")
        .subcommand(
            Command::new("dir")
                .about("Set the download directory")
                .arg(Arg::new("path").required(true)),
        )
}

pub async fn run_download(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("dir", sub)) => {
            let path_arg = sub.get_one::<String>("path").cloned().unwrap_or_default();
            let dir = match std::path::absolute(&path_arg) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(e) => {
                    eprintln!("Error: invalid path: {e}");
                    std::process::exit(1);
                }
            };
            let mut args = Map::new();
            args.insert("path".to_string(), Value::from(dir));
            match daemon_call("browser_download_set_dir", args, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
        }
        // No subcommand: mirror Go's `cmd.Help()`.
        _ => {
            let _ = download_command().print_help();
            println!();
        }
    }
}
