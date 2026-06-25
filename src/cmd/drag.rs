use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn drag_command() -> Command {
    Command::new("drag")
        .about("Drag from one element to another")
        .arg(
            Arg::new("source")
                .required(true)
                .num_args(1)
                .help("CSS selector (or map ref) for the element to drag"),
        )
        .arg(
            Arg::new("target")
                .required(true)
                .num_args(1)
                .help("CSS selector (or map ref) for the drop target"),
        )
        .after_help(examples(&[
            (
                "drag \".draggable\" \".dropzone\"",
                "Drag element to drop target",
            ),
            ("drag @e1 @e3", "Drag using map refs"),
        ]))
}

pub async fn run_drag(source: String, target: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("source".to_string(), Value::from(source));
    args.insert("target".to_string(), Value::from(target));
    match daemon_call("browser_drag", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
