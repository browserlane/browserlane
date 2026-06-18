use clap::{Arg, ArgAction, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn screenshot_command() -> Command {
    Command::new("screenshot")
        .about("Capture a screenshot (optionally navigate to URL first)")
        .arg(Arg::new("url").num_args(0..=1))
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .default_value("screenshot.png")
                .help("Output file path"),
        )
        .arg(
            Arg::new("full-page")
                .long("full-page")
                .action(ArgAction::SetTrue)
                .help("Capture the full page instead of just the viewport"),
        )
        .arg(
            Arg::new("annotate")
                .long("annotate")
                .action(ArgAction::SetTrue)
                .help("Annotate interactive elements with numbered labels"),
        )
}

pub async fn run_screenshot(
    url: Option<String>,
    output: String,
    full_page: bool,
    annotate: bool,
    headless: bool,
    json_output: bool,
) {
    if let Some(url) = url {
        let mut m = Map::new();
        m.insert("url".to_string(), Value::from(url));
        if let Err(e) = daemon_call("browser_navigate", m, headless).await {
            print_error(&e, json_output);
        }
    }

    let mut args = Map::new();
    args.insert("filename".to_string(), Value::from(output));
    if full_page {
        args.insert("fullPage".to_string(), Value::from(true));
    }
    if annotate {
        args.insert("annotate".to_string(), Value::from(true));
    }

    match daemon_call("browser_screenshot", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
