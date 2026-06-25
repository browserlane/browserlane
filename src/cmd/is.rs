use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::helpers::print_check;
use super::output::{print_error, print_result};
use crate::agent::ToolsCallResult;

pub fn is_command() -> Command {
    Command::new("is")
        .about("Check element state (visible, enabled, checked, actionable)")
        // No subcommand prints this parent's help natively (cobra's `cmd.Help()`).
        .arg_required_else_help(true)
        .subcommand(
            Command::new("visible")
                .about("Check if an element is visible on the page")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element to check"))
                .after_help(examples(&[("is visible \"h1\"", "Prints true or false")])),
        )
        .subcommand(
            Command::new("enabled")
                .about("Check if an element is enabled")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element to check"))
                .after_help(examples(&[(
                    "is enabled \"button[type=submit]\"",
                    "Prints true or false",
                )])),
        )
        .subcommand(
            Command::new("checked")
                .about("Check if a checkbox or radio is checked")
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the checkbox or radio to check"))
                .after_help(examples(&[(
                    "is checked \"input[type=checkbox]\"",
                    "Prints true or false",
                )])),
        )
        .subcommand(
            Command::new("actionable")
                .about("Check actionability of an element (Visible, Stable, ReceivesEvents, Enabled, Editable)")
                .arg(Arg::new("url").required(true).num_args(1).help("URL to navigate to before checking"))
                .arg(Arg::new("selector").required(true).num_args(1).help("CSS selector of the element to check"))
                .after_help(examples(&[(
                    "is actionable https://example.com \"a\"",
                    "Output:\n  # Checking actionability for selector: a\n  # ✓ Visible: true\n  # ✓ Stable: true\n  # ✓ ReceivesEvents: true\n  # ✓ Enabled: true\n  # ✗ Editable: false",
                )])),
        )
}

pub async fn run_is(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("visible", sub)) => {
            run_state_check("browser_is_visible", sub, headless, json_output).await;
        }
        Some(("enabled", sub)) => {
            run_state_check("browser_is_enabled", sub, headless, json_output).await;
        }
        Some(("checked", sub)) => {
            run_state_check("browser_is_checked", sub, headless, json_output).await;
        }
        Some(("actionable", sub)) => {
            run_actionable(sub, headless, json_output).await;
        }
        _ => {
            // Parent `is` with no subcommand prints help (Go: cmd.Help()).
            let _ = is_command().print_help();
            println!();
        }
    }
}

/// Sends a `{selector}` tool call and prints the result (visible/enabled/checked).
async fn run_state_check(tool: &str, sub: &ArgMatches, headless: bool, json_output: bool) {
    let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
    let mut args = Map::new();
    args.insert("selector".to_string(), Value::from(selector));
    match daemon_call(tool, args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}

async fn run_actionable(sub: &ArgMatches, headless: bool, json_output: bool) {
    let url = sub.get_one::<String>("url").cloned().unwrap_or_default();
    let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();

    // Navigate to URL.
    let mut nav = Map::new();
    nav.insert("url".to_string(), Value::from(url));
    if let Err(e) = daemon_call("browser_navigate", nav, headless).await {
        print_error(&e, json_output);
    }

    println!("\nChecking actionability for selector: {selector}");

    // Evaluate actionability script.
    let quoted = serde_json::to_string(&selector).unwrap_or_else(|_| format!("\"{selector}\""));
    let script = format!(
        r#"(() => {{
				const selector = {quoted};
				const el = document.querySelector(selector);
				if (!el) return JSON.stringify({{ error: 'element not found' }});

				const rect = el.getBoundingClientRect();
				const style = window.getComputedStyle(el);
				const visible = rect.width > 0 && rect.height > 0 &&
					style.visibility !== 'hidden' && style.display !== 'none';

				const cx = rect.x + rect.width/2, cy = rect.y + rect.height/2;
				const hit = document.elementFromPoint(cx, cy);
				const receivesEvents = hit && (el === hit || el.contains(hit));

				let enabled = true;
				if (el.disabled === true) enabled = false;
				else if (el.getAttribute('aria-disabled') === 'true') enabled = false;
				else {{
					const fs = el.closest('fieldset[disabled]');
					if (fs) {{ const legend = fs.querySelector('legend'); if (!legend || !legend.contains(el)) enabled = false; }}
				}}

				let editable = enabled && !el.readOnly && el.getAttribute('aria-readonly') !== 'true';
				if (editable) {{
					const tag = el.tagName.toLowerCase();
					if (tag === 'input') {{
						const t = (el.type || 'text').toLowerCase();
						editable = ['text','password','email','number','search','tel','url'].includes(t);
					}} else if (tag !== 'textarea' && !el.isContentEditable) {{
						editable = false;
					}}
				}}

				return JSON.stringify({{ visible, stable: true, receivesEvents, enabled, editable }});
			}})()"#
    );

    let mut eval_args = Map::new();
    eval_args.insert("expression".to_string(), Value::from(script));
    let result = match daemon_call("browser_evaluate", eval_args, headless).await {
        Ok(r) => r,
        Err(e) => print_error(&e, json_output),
    };

    // Parse the result.
    let result_text = first_text(&result);

    let action: ActionResult = match serde_json::from_str(&result_text) {
        Ok(a) => a,
        Err(e) => print_error(&anyhow::anyhow!("failed to parse actionability result: {e}"), json_output),
    };
    if !action.error.is_empty() {
        print_error(&anyhow::anyhow!("{}", action.error), json_output);
    }

    print_check("Visible", action.visible);
    print_check("Stable", action.stable);
    print_check("ReceivesEvents", action.receives_events);
    print_check("Enabled", action.enabled);
    print_check("Editable", action.editable);
}

/// Returns the first text content from a tool result.
fn first_text(result: &ToolsCallResult) -> String {
    for c in &result.content {
        if c.content_type == "text" {
            return c.text.clone();
        }
    }
    String::new()
}

#[derive(Debug, Default, Deserialize)]
struct ActionResult {
    #[serde(default)]
    visible: bool,
    #[serde(default)]
    stable: bool,
    #[serde(default, rename = "receivesEvents")]
    receives_events: bool,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    editable: bool,
    #[serde(default)]
    error: String,
}
