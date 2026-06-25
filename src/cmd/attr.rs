use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn attr_command() -> Command {
    Command::new("attr")
        .about("Get an HTML attribute value from an element")
        .arg(
            Arg::new("selector")
                .required(true)
                .num_args(1)
                .help("CSS selector for the element"),
        )
        .arg(
            Arg::new("attribute")
                .required(true)
                .num_args(1)
                .help("Attribute name to read (e.g. href, src)"),
        )
        .after_help(examples(&[
            ("attr \"a\" \"href\"", "Get the href of the first link"),
            ("attr \"img\" \"src\"", "Get the image source URL"),
        ]))
}

pub async fn run_attr(selector: String, attribute: String, headless: bool, json_output: bool) {
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    args.insert("attribute".to_string(), Value::from(attribute));

    match daemon_call("browser_get_attribute", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
