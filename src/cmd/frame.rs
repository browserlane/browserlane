use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn frame_command() -> Command {
    Command::new("frame")
        .about("Find a frame by name or URL substring")
        .arg(Arg::new("nameOrUrl").required(true).help("Frame name or URL substring to match"))
        .after_help(examples(&[
            ("frame \"myIframe\"", "Find frame by name"),
            ("frame \"example.com\"", "Find frame by URL substring"),
        ]))
}

pub async fn run_frame(matches: &ArgMatches, headless: bool, json_output: bool) {
    let name_or_url = matches.get_one::<String>("nameOrUrl").cloned().unwrap_or_default();
    let mut args = Map::new();
    args.insert("nameOrUrl".to_string(), Value::from(name_or_url));
    match daemon_call("browser_frame", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
