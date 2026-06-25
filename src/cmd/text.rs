use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::find::is_url;
use super::output::{print_error, print_result};

pub fn text_command() -> Command {
    Command::new("text")
        .about("Get text content of the page or an element")
        .arg(
            Arg::new("args")
                .num_args(0..=2)
                .help("CSS selector, or a URL to navigate to first (optionally followed by a selector)"),
        )
        .after_help(examples(&[
            ("text", "Get all page text"),
            ("text \"h1\"", "Get text of a specific element"),
            ("text https://example.com", "Navigate then get all page text"),
            (
                "text https://example.com \"h1\"",
                "Navigate then get element text",
            ),
        ]))
}

pub async fn run_text(args: Vec<String>, headless: bool, json_output: bool) {
    let mut tool_args = Map::new();

    if args.len() == 2 {
        // text <url> <selector> — navigate first.
        if let Err(e) = navigate(&args[0], headless).await {
            print_error(&e, json_output);
        }
        tool_args.insert("selector".to_string(), Value::from(args[1].clone()));
    } else if args.len() == 1 {
        if is_url(&args[0]) {
            // text <url> — navigate then get all page text.
            if let Err(e) = navigate(&args[0], headless).await {
                print_error(&e, json_output);
            }
        } else {
            // text <selector> — get element text on current page.
            tool_args.insert("selector".to_string(), Value::from(args[0].clone()));
        }
    }

    match daemon_call("browser_get_text", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}

/// Sends a `browser_navigate` tool call to the daemon.
async fn navigate(url: &str, headless: bool) -> anyhow::Result<()> {
    let mut m = Map::new();
    m.insert("url".to_string(), Value::from(url.to_string()));
    daemon_call("browser_navigate", m, headless).await.map(|_| ())
}
