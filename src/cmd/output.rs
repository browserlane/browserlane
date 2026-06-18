use serde::Serialize;

use crate::agent::ToolsCallResult;
use crate::process;

/// Output format for --json mode.
#[derive(Serialize)]
struct JsonEnvelope {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
}

/// Prints a tool call result, respecting --json mode.
pub fn print_result(result: &ToolsCallResult, json_output: bool) {
    if json_output {
        let text = extract_text(result);
        print_json(&JsonEnvelope {
            ok: true,
            result: Some(text),
            error: String::new(),
        });
        return;
    }

    // Human-readable: just print the text content.
    for c in &result.content {
        if c.content_type == "text" && !c.text.is_empty() {
            println!("{}", c.text);
        }
    }
}

/// Prints an error, respecting --json mode. Always exits the process.
pub fn print_error(err: &anyhow::Error, json_output: bool) -> ! {
    if json_output {
        print_json(&JsonEnvelope {
            ok: false,
            result: None,
            error: err.to_string(),
        });
        process::kill_all();
        std::process::exit(1);
    }

    eprintln!("Error: {err}");
    process::kill_all();
    std::process::exit(1);
}

/// Marshals and prints a value as a single JSON line.
fn print_json<T: Serialize>(v: &T) {
    match serde_json::to_string(v) {
        Ok(data) => println!("{data}"),
        Err(e) => eprintln!("Error marshaling JSON: {e}"),
    }
}

/// Marshals and prints a JSON value as a single line (used by `daemon status --json`).
pub(crate) fn print_json_value(v: &serde_json::Value) {
    print_json(v);
}

/// Returns the first text content from a result.
pub(crate) fn extract_text(result: &ToolsCallResult) -> String {
    for c in &result.content {
        if c.content_type == "text" {
            return c.text.clone();
        }
    }
    String::new()
}
