use clap::{Arg, ArgAction, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::find::is_url;
use super::output::{print_error, print_result};

pub fn html_command() -> Command {
    Command::new("html")
        .about("Get HTML content of the page or an element")
        .arg(
            Arg::new("args")
                .num_args(0..=2)
                .help("CSS selector, or a URL to navigate to first (optionally followed by a selector)"),
        )
        .arg(
            Arg::new("outer")
                .long("outer")
                .action(ArgAction::SetTrue)
                .help("Return outerHTML instead of innerHTML"),
        )
        .after_help(examples(&[
            ("html", "Get full page HTML"),
            ("html \"div.content\"", "Get innerHTML of a specific element"),
            (
                "html \"div.content\" --outer",
                "Get outerHTML of a specific element",
            ),
            ("html https://example.com \"h1\"", "Navigate then get element HTML"),
        ]))
}

pub async fn run_html(args: Vec<String>, outer: bool, headless: bool, json_output: bool) {
    let mut tool_args = Map::new();
    if outer {
        tool_args.insert("outer".to_string(), Value::from(true));
    }

    if args.len() == 2 {
        // html <url> <selector> — navigate first.
        if let Err(e) = navigate(&args[0], headless).await {
            print_error(&e, json_output);
        }
        tool_args.insert("selector".to_string(), Value::from(args[1].clone()));
    } else if args.len() == 1 {
        if is_url(&args[0]) {
            // html <url> — navigate then get full page HTML.
            if let Err(e) = navigate(&args[0], headless).await {
                print_error(&e, json_output);
            }
        } else {
            // html <selector> — get element HTML on current page.
            tool_args.insert("selector".to_string(), Value::from(args[0].clone()));
        }
    }

    match daemon_call("browser_get_html", tool_args, headless).await {
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
