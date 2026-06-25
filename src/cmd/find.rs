use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn find_command() -> Command {
    let semantic = |name: &'static str,
                    arg: &'static str,
                    about: &'static str,
                    example: (&'static str, &'static str)| {
        Command::new(name)
            .about(about)
            .arg(Arg::new(arg).required(true).num_args(1).help(about))
            .after_help(examples(&[example]))
    };

    Command::new("find")
        .about("Find elements by CSS selector or semantic locator")
        .arg(
            Arg::new("selector")
                .num_args(0..=2)
                .help("CSS selector (optionally preceded by a URL to navigate first)"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Find all matching elements"),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .default_value("10")
                .value_parser(clap::value_parser!(i64))
                .help("Maximum number of elements to return (with --all)"),
        )
        .after_help(examples(&[
            ("find \"a\"", "→ @e1 [a] \"More information...\""),
            ("find \"a\" --all", "→ @e1 [a] \"Home\"  @e2 [a] \"About\"  ..."),
            ("find text \"Sign In\"", "→ @e1 [button] \"Sign In\""),
            ("find role button", "→ @e1 [button] \"Submit\""),
            (
                "find role heading --name \"Example\"",
                "Find heading with accessible name \"Example\"",
            ),
        ]))
        .subcommand(semantic(
            "text",
            "text",
            "Find element by text content",
            ("find text \"Sign In\"", "→ @e1 [button] \"Sign In\""),
        ))
        .subcommand(
            Command::new("role")
                .about("Find element by ARIA role")
                .arg(Arg::new("role").required(true).num_args(1).help("ARIA role to match"))
                .arg(
                    Arg::new("name")
                        .long("name")
                        .default_value("")
                        .help("Accessible name filter"),
                )
                .after_help(examples(&[
                    ("find role button", "→ @e1 [button] \"Submit\""),
                    (
                        "find role heading --name \"Example\"",
                        "Find heading with accessible name \"Example\"",
                    ),
                ])),
        )
        .subcommand(semantic(
            "label",
            "label",
            "Find input by associated label text",
            (
                "find label \"Email\"",
                "→ @e1 [input type=\"email\"] placeholder=\"Email\"",
            ),
        ))
        .subcommand(semantic(
            "placeholder",
            "placeholder",
            "Find element by placeholder attribute",
            (
                "find placeholder \"Search...\"",
                "→ @e1 [input] placeholder=\"Search...\"",
            ),
        ))
        .subcommand(semantic(
            "testid",
            "testid",
            "Find element by data-testid attribute",
            (
                "find testid \"submit-btn\"",
                "→ @e1 [button] data-testid=\"submit-btn\"",
            ),
        ))
        .subcommand(semantic(
            "xpath",
            "expression",
            "Find element by XPath expression",
            ("find xpath \"//div[@class='main']\"", "→ @e1 [div.main] ..."),
        ))
        .subcommand(semantic(
            "alt",
            "alt",
            "Find element by alt attribute",
            ("find alt \"Logo\"", ""),
        ))
        .subcommand(semantic(
            "title",
            "title",
            "Find element by title attribute",
            ("find title \"Close\"", ""),
        ))
}

pub async fn run_find(matches: &ArgMatches, headless: bool, json_output: bool) {
    // Semantic locator subcommands.
    let semantic_arg = |sub: &ArgMatches, key: &str| -> Map<String, Value> {
        let mut m = Map::new();
        if let Some(v) = sub.get_one::<String>(key) {
            m.insert(key.to_string(), Value::from(v.clone()));
        }
        m
    };

    let tool_args: Map<String, Value> = match matches.subcommand() {
        Some(("text", sub)) => semantic_arg(sub, "text"),
        Some(("role", sub)) => {
            let mut m = Map::new();
            m.insert("role".to_string(), Value::from(sub.get_one::<String>("role").cloned().unwrap_or_default()));
            let name = sub.get_one::<String>("name").cloned().unwrap_or_default();
            if !name.is_empty() {
                m.insert("text".to_string(), Value::from(name));
            }
            m
        }
        Some(("label", sub)) => semantic_arg(sub, "label"),
        Some(("placeholder", sub)) => semantic_arg(sub, "placeholder"),
        Some(("testid", sub)) => semantic_arg(sub, "testid"),
        Some(("xpath", sub)) => {
            let mut m = Map::new();
            m.insert("xpath".to_string(), Value::from(sub.get_one::<String>("expression").cloned().unwrap_or_default()));
            m
        }
        Some(("alt", sub)) => semantic_arg(sub, "alt"),
        Some(("title", sub)) => semantic_arg(sub, "title"),
        Some((_, _)) | None => {
            // Top-level: find by CSS selector (positional), optional --all.
            let args: Vec<String> = matches
                .get_many::<String>("selector")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            if args.is_empty() {
                eprintln!("Error: requires a CSS selector or use a subcommand (text, role, label, etc.)");
                std::process::exit(1);
            }

            let mut tool_args = Map::new();
            if args.len() == 2 && is_url(&args[0]) {
                if let Err(e) = daemon_call(
                    "browser_navigate",
                    {
                        let mut m = Map::new();
                        m.insert("url".to_string(), Value::from(args[0].clone()));
                        m
                    },
                    headless,
                )
                .await
                {
                    print_error(&e, json_output);
                }
                tool_args.insert("selector".to_string(), Value::from(args[1].clone()));
            } else {
                tool_args.insert("selector".to_string(), Value::from(args[0].clone()));
            }

            if matches.get_flag("all") {
                let limit = *matches.get_one::<i64>("limit").unwrap_or(&10);
                tool_args.insert("limit".to_string(), Value::from(limit));
                match daemon_call("browser_find_all", tool_args, headless).await {
                    Ok(result) => print_result(&result, json_output),
                    Err(e) => print_error(&e, json_output),
                }
                return;
            }

            match daemon_call("browser_find", tool_args, headless).await {
                Ok(result) => print_result(&result, json_output),
                Err(e) => print_error(&e, json_output),
            }
            return;
        }
    };

    match daemon_call("browser_find", tool_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}

/// Returns true if the string looks like a URL.
pub fn is_url(s: &str) -> bool {
    s.len() > 8 && (s.starts_with("http://") || s.starts_with("https://"))
}
