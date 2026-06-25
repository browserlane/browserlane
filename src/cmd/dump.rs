//! The hidden `__dump` command: introspects the live clap `Command` tree and
//! emits it as JSON on stdout.
//!
//! This is the machine-readable mirror of the help surface. The docs generator
//! (Phase 3+) consumes it instead of scraping `--help`, so the website and the
//! CLI share one source of truth — the clap tree built by `build_cli()`.
//!
//! It is `.hide(true)` (not shown in help, not suggested) and introspection-only
//! — no browser, no daemon, no I/O beyond the JSON write — so it is fast and
//! safe to call in CI and build scripts.
//!
//! Sparse fields (missing `long_about`, empty `after_help`, `category: "Other"`)
//! are expected for un-migrated commands and auto-fill as Phase 3 enriches each
//! command — the introspection here needs no changes per command.

use clap::{Arg, Command};
use serde_json::{json, Map, Value};

use super::category::category_for;

/// The hidden `__dump` subcommand definition. Registered in `build_cli()` behind
/// `.hide(true)` so it never appears in help or completion.
pub fn dump_command() -> Command {
    Command::new("__dump")
        .hide(true)
        .about("Dump the CLI command tree as JSON (internal; for docs generation)")
}

/// Walks `root`'s subcommands and prints the command tree as pretty JSON.
///
/// `root` is the command built by `build_cli()`, so the dump reflects exactly
/// what the CLI exposes. The top-level object carries the program identity plus
/// the root's own global args, then a `commands` array of every visible
/// subcommand (recursively). `__dump` itself and other hidden commands are
/// skipped so the output matches the user-visible surface.
pub fn run_dump(root: &Command) {
    let commands: Vec<Value> = root
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| command_json(c, &[]))
        .collect();

    let doc = json!({
        "program": root.get_name(),
        "version": crate::VERSION,
        "about": opt_str(root.get_about().map(|s| s.to_string())),
        // The root's global flags, derived from the same args the grouped help
        // renders, so downstream consumers see them without re-deriving.
        "global_args": root
            .get_arguments()
            .filter(|a| a.is_global_set())
            .map(arg_json)
            .collect::<Vec<_>>(),
        "commands": commands,
    });

    // Pretty-print; fall back to compact only if pretty somehow fails.
    match serde_json::to_string_pretty(&doc) {
        Ok(s) => println!("{s}"),
        Err(_) => println!("{doc}"),
    }
}

/// Serializes a single command (and, recursively, its subcommands) to JSON.
/// `parents` is the chain of ancestor names, so `path` is the full space-joined
/// command path (e.g. `["find"]` for the `role` subcommand yields `find role`).
fn command_json(cmd: &Command, parents: &[&str]) -> Value {
    let name = cmd.get_name();

    // Full path = ancestors + this name, space-joined ("find role").
    let mut path_parts: Vec<&str> = parents.to_vec();
    path_parts.push(name);
    let path = path_parts.join(" ");

    // Recurse into visible child commands, threading this command into the
    // parent chain. clap's auto-injected `help` subcommand is skipped.
    let child_parents: Vec<&str> = path_parts.clone();
    let subcommands: Vec<Value> = cmd
        .get_subcommands()
        .filter(|c| !c.is_hide_set() && c.get_name() != "help")
        .map(|c| command_json(c, &child_parents))
        .collect();

    // Per-command args, excluding clap's auto help/version flags (they are not
    // part of the authored surface and would be noise in the docs).
    let args: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| {
            let id = a.get_id().as_str();
            id != "help" && id != "version"
        })
        .map(arg_json)
        .collect();

    json!({
        "name": name,
        "path": path,
        "category": category_name(category_for(name)),
        "about": opt_str(cmd.get_about().map(|s| s.to_string())),
        "long_about": opt_str(cmd.get_long_about().map(|s| s.to_string())),
        // Raw after_help (the Examples: block) so downstream tools have the
        // examples verbatim. Includes the live program name already spliced in.
        "after_help": opt_str(cmd.get_after_help().map(|s| s.to_string())),
        "args": args,
        "subcommands": subcommands,
    })
}

/// Serializes a single argument's introspectable shape to JSON.
fn arg_json(arg: &Arg) -> Value {
    let num_args = arg.get_num_args().map(|r| {
        let mut m = Map::new();
        m.insert("min".to_string(), json!(r.min_values()));
        // `max_values()` returns usize::MAX for unbounded ranges; surface that
        // as null so consumers don't choke on a giant sentinel integer.
        let max = r.max_values();
        m.insert(
            "max".to_string(),
            if max == usize::MAX { Value::Null } else { json!(max) },
        );
        Value::Object(m)
    });

    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();

    // Whether the arg consumes a value, derived from its action: `Set`/`Append`
    // take a value (`-o file`, `--name x`, and every positional), while
    // `SetTrue`/`SetFalse`/`Count` are switches (`--full-page`). Without this,
    // value-taking flags and switches both emit `num_args: null` and are
    // indistinguishable to downstream consumers (the docs generator). A
    // positional is told apart from a value-taking option by having neither a
    // `long` nor a `short`.
    let takes_value = arg.get_action().takes_values();

    json!({
        "name": arg.get_id().as_str(),
        "short": arg.get_short().map(|c| c.to_string()),
        "long": arg.get_long(),
        "help": opt_str(arg.get_help().map(|s| s.to_string())),
        "required": arg.is_required_set(),
        "default": defaults,
        "takes_value": takes_value,
        "num_args": num_args,
    })
}

/// `Option<String>` -> JSON string or null (keeps sparse fields explicit).
fn opt_str(s: Option<String>) -> Value {
    match s {
        Some(v) => Value::String(v),
        None => Value::Null,
    }
}

/// Stable machine name for a category (matches the variant, not the header).
fn category_name(cat: super::category::Category) -> &'static str {
    use super::category::Category::*;
    match cat {
        Navigation => "Navigation",
        Interaction => "Interaction",
        InspectPage => "InspectPage",
        Capture => "Capture",
        BrowserState => "BrowserState",
        Scripting => "Scripting",
        SessionDaemon => "SessionDaemon",
        AgentMcp => "AgentMcp",
        SetupDiagnostics => "SetupDiagnostics",
        Other => "Other",
    }
}
