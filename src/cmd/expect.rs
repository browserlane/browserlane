use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

/// The `contains`/`equals` comparison as a validated positional arg, shared by
/// the url/title/text/value subcommands.
fn operator_arg() -> Arg {
    Arg::new("operator")
        .required(true)
        .num_args(1)
        .value_parser(["contains", "equals"])
        .help("Comparison: contains (substring) or equals (exact match)")
}

pub fn expect_command() -> Command {
    Command::new("expect")
        .about("Assert page state; exits 0 on pass, 1 on failure")
        // No subcommand prints this parent's help natively (like `is`).
        .arg_required_else_help(true)
        .subcommand(
            Command::new("url")
                .about("Assert the current page URL")
                .arg(operator_arg())
                .arg(Arg::new("text").required(true).num_args(1).help("Expected URL or URL substring"))
                .after_help(examples(&[
                    ("expect url contains \"/dashboard\"", "Pass if the URL contains /dashboard"),
                    ("expect url equals \"https://example.com/\"", "Pass if the URL matches exactly"),
                ])),
        )
        .subcommand(
            Command::new("title")
                .about("Assert the current page title")
                .arg(operator_arg())
                .arg(Arg::new("text").required(true).num_args(1).help("Expected title or title substring"))
                .after_help(examples(&[(
                    "expect title contains \"Dashboard\"",
                    "Pass if the title contains Dashboard",
                )])),
        )
        .subcommand(
            Command::new("text")
                .about("Assert the text content of the page or an element")
                .arg(operator_arg())
                .arg(Arg::new("text").required(true).num_args(1).help("Expected text or text substring"))
                .arg(
                    Arg::new("selector")
                        .long("selector")
                        .num_args(1)
                        .help("CSS selector to scope the check to one element (defaults to full page text)"),
                )
                .after_help(examples(&[
                    ("expect text contains \"Welcome back\"", "Pass if the page text contains it"),
                    (
                        "expect text equals \"Logout\" --selector \"#nav button\"",
                        "Pass if the element's text matches exactly",
                    ),
                ])),
        )
        .subcommand(
            Command::new("visible")
                .about("Assert an element is visible")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element"))
                .after_help(examples(&[("expect visible \"#save\"", "Pass if the element is visible")])),
        )
        .subcommand(
            Command::new("hidden")
                .about("Assert an element is absent or not visible")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element"))
                .after_help(examples(&[(
                    "expect hidden \".spinner\"",
                    "Pass if the element is missing or not visible",
                )])),
        )
        .subcommand(
            Command::new("enabled")
                .about("Assert an element is enabled")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element"))
                .after_help(examples(&[(
                    "expect enabled \"button[type=submit]\"",
                    "Pass if the button is enabled",
                )])),
        )
        .subcommand(
            Command::new("checked")
                .about("Assert a checkbox or radio is checked (or unchecked with --not)")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the checkbox or radio"))
                .arg(
                    Arg::new("not")
                        .long("not")
                        .action(ArgAction::SetTrue)
                        .help("Assert the element is NOT checked"),
                )
                .after_help(examples(&[
                    ("expect checked \"#terms\"", "Pass if the box is checked"),
                    ("expect checked \"#terms\" --not", "Pass if the box is unchecked"),
                ])),
        )
        .subcommand(
            Command::new("value")
                .about("Assert the value of a form element")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the form element"))
                .arg(operator_arg())
                .arg(Arg::new("text").required(true).num_args(1).help("Expected value or value substring"))
                .after_help(examples(&[(
                    "expect value \"input[name=email]\" equals \"a@b.com\"",
                    "Pass if the input holds exactly that value",
                )])),
        )
        .subcommand(
            Command::new("count")
                .about("Assert the number of elements matching a selector")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector to count matches for"))
                .arg(
                    Arg::new("number")
                        .required(true)
                        .num_args(1)
                        .value_parser(clap::value_parser!(i64))
                        .help("Expected number of matches"),
                )
                .after_help(examples(&[("expect count \"li.item\" 3", "Pass if exactly 3 items match")])),
        )
        .subcommand(
            Command::new("js")
                .about("Assert a JavaScript expression evaluates truthy")
                .arg(
                    Arg::new("expression")
                        .required(true)
                        .num_args(1)
                        .help("JS expression; false, null, empty string, and 0 fail"),
                )
                .after_help(examples(&[(
                    "expect js \"document.querySelectorAll('.error').length === 0\"",
                    "Pass if no error elements are on the page",
                )])),
        )
}

pub async fn run_expect(matches: &ArgMatches, headless: bool, json_output: bool) {
    // Translate the ergonomic subcommands into the generic browser_expect
    // arguments; all assertion logic lives in the MCP handler.
    let mut args = Map::new();
    match matches.subcommand() {
        Some((target @ ("url" | "title"), sub)) => {
            args.insert("target".to_string(), Value::from(target));
            args.insert("operator".to_string(), Value::from(str_arg(sub, "operator")));
            args.insert("expected".to_string(), Value::from(str_arg(sub, "text")));
        }
        Some(("text", sub)) => {
            args.insert("target".to_string(), Value::from("text"));
            args.insert("operator".to_string(), Value::from(str_arg(sub, "operator")));
            args.insert("expected".to_string(), Value::from(str_arg(sub, "text")));
            if let Some(sel) = sub.get_one::<String>("selector") {
                args.insert("selector".to_string(), Value::from(sel.clone()));
            }
        }
        Some((target @ ("visible" | "hidden" | "enabled"), sub)) => {
            args.insert("target".to_string(), Value::from(target));
            args.insert("selector".to_string(), Value::from(str_arg(sub, "selector")));
        }
        Some(("checked", sub)) => {
            args.insert("target".to_string(), Value::from("checked"));
            args.insert("selector".to_string(), Value::from(str_arg(sub, "selector")));
            if sub.get_flag("not") {
                args.insert("negate".to_string(), Value::from(true));
            }
        }
        Some(("value", sub)) => {
            args.insert("target".to_string(), Value::from("value"));
            args.insert("selector".to_string(), Value::from(str_arg(sub, "selector")));
            args.insert("operator".to_string(), Value::from(str_arg(sub, "operator")));
            args.insert("expected".to_string(), Value::from(str_arg(sub, "text")));
        }
        Some(("count", sub)) => {
            args.insert("target".to_string(), Value::from("count"));
            args.insert("selector".to_string(), Value::from(str_arg(sub, "selector")));
            args.insert(
                "expected".to_string(),
                Value::from(sub.get_one::<i64>("number").copied().unwrap_or_default()),
            );
        }
        Some(("js", sub)) => {
            args.insert("target".to_string(), Value::from("js"));
            args.insert("expression".to_string(), Value::from(str_arg(sub, "expression")));
        }
        _ => {
            // Parent `expect` with no subcommand prints help (like `is`).
            let _ = expect_command().print_help();
            println!();
            return;
        }
    }

    match daemon_call("browser_expect", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}

/// Reads a required positional string arg (clap guarantees presence).
fn str_arg(sub: &ArgMatches, name: &str) -> String {
    sub.get_one::<String>(name).cloned().unwrap_or_default()
}
