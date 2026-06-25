use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::examples::examples;
use super::output::{print_error, print_result};

pub fn record_command() -> Command {
    let start = Command::new("start")
        .about("Start a recording")
        .arg(
            Arg::new("screenshots")
                .long("screenshots")
                .num_args(0..=1)
                .default_value("true")
                .default_missing_value("true")
                .value_parser(clap::value_parser!(bool))
                .help("Capture screenshots after each action"),
        )
        .arg(
            Arg::new("snapshots")
                .long("snapshots")
                .action(ArgAction::SetTrue)
                .help("Capture HTML snapshots"),
        )
        .arg(
            Arg::new("sources")
                .long("sources")
                .action(ArgAction::SetTrue)
                .help("Include source information"),
        )
        .arg(
            Arg::new("bidi")
                .long("bidi")
                .action(ArgAction::SetTrue)
                .help("Record raw BiDi commands in the recording"),
        )
        .arg(Arg::new("name").long("name").help("Name for the recording"))
        .arg(
            Arg::new("title")
                .long("title")
                .help("Title shown in trace viewer (defaults to name)"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .default_value("jpeg")
                .help("Screenshot format: jpeg or png"),
        )
        .arg(
            Arg::new("quality")
                .long("quality")
                .value_parser(clap::value_parser!(f64))
                .default_value("0.5")
                .help("JPEG quality 0.0-1.0 (ignored for png)"),
        )
        .after_help(examples(&[
            ("record start", "Start recording with screenshots (default)"),
            ("record start --screenshots=false", "Record without screenshots"),
            (
                "record start --snapshots",
                "Record with screenshots and HTML snapshots",
            ),
            (
                "record start --format png",
                "Use PNG format instead of JPEG (larger files, lossless)",
            ),
            (
                "record start --quality 0.1",
                "Lower JPEG quality for smaller recording files",
            ),
            (
                "record start --title \"Login Flow\"",
                "Set a title shown in the trace viewer",
            ),
        ]));

    let stop = Command::new("stop")
        .about("Stop recording and save")
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file path (default: record.zip)"),
        )
        .after_help(examples(&[
            ("record stop", "Save recording to record.zip"),
            ("record stop -o my-recording.zip", "Save recording to custom path"),
        ]));

    let group = Command::new("group")
        .about("Manage recording groups")
        // No subcommand prints this sub-parent's help natively (cobra's `cmd.Help()`).
        .arg_required_else_help(true)
        .subcommand(
            Command::new("start")
                .about("Start a named group in the recording")
                .arg(Arg::new("name").required(true).help("Name for the group"))
                .after_help(examples(&[(
                    "record group start \"Login\"",
                    "Groups nest actions in the trace viewer",
                )])),
        )
        .subcommand(
            Command::new("stop")
                .about("End the current recording group")
                .after_help(examples(&[("record group stop", "")])),
        );

    let chunk = Command::new("chunk")
        .about("Manage recording chunks")
        // No subcommand prints this sub-parent's help natively (cobra's `cmd.Help()`).
        .arg_required_else_help(true)
        .subcommand(
            Command::new("start")
                .about("Start a new chunk within the current recording")
                .arg(Arg::new("name").long("name").help("Name for the chunk"))
                .arg(
                    Arg::new("title")
                        .long("title")
                        .help("Title shown in trace viewer"),
                )
                .after_help(examples(&[
                    (
                        "record chunk start",
                        "Start a new chunk (for splitting long recordings)",
                    ),
                    (
                        "record chunk start --name \"part2\" --title \"Checkout Flow\"",
                        "",
                    ),
                ])),
        )
        .subcommand(
            Command::new("stop")
                .about("Package current chunk into a ZIP file (recording stays active)")
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file path (default: chunk.zip)"),
                )
                .after_help(examples(&[
                    ("record chunk stop", "Save chunk to chunk.zip"),
                    ("record chunk stop -o part1.zip", ""),
                ])),
        );

    Command::new("record")
        .about("Record browser sessions (screenshots and snapshots)")
        // No subcommand prints this parent's help natively (cobra's `cmd.Help()`).
        .arg_required_else_help(true)
        .subcommand(start)
        .subcommand(stop)
        .subcommand(group)
        .subcommand(chunk)
}

pub async fn run_record(matches: &ArgMatches, headless: bool, json_output: bool) {
    match matches.subcommand() {
        Some(("start", sub)) => {
            let screenshots = sub.get_one::<bool>("screenshots").copied().unwrap_or(true);
            let snapshots = sub.get_flag("snapshots");
            let bidi = sub.get_flag("bidi");
            let name = sub.get_one::<String>("name").cloned().unwrap_or_default();
            let title = sub.get_one::<String>("title").cloned().unwrap_or_default();
            let sources = sub.get_flag("sources");
            let format = sub.get_one::<String>("format").cloned().unwrap_or_else(|| "jpeg".to_string());
            let quality = sub.get_one::<f64>("quality").copied().unwrap_or(0.5);

            let mut call_args = Map::new();
            if !name.is_empty() {
                call_args.insert("name".to_string(), Value::from(name));
            }
            if !title.is_empty() {
                call_args.insert("title".to_string(), Value::from(title));
            }
            call_args.insert("screenshots".to_string(), Value::from(screenshots));
            if snapshots {
                call_args.insert("snapshots".to_string(), Value::from(true));
            }
            if sources {
                call_args.insert("sources".to_string(), Value::from(true));
            }
            if bidi {
                call_args.insert("bidi".to_string(), Value::from(true));
            }
            if format != "jpeg" {
                call_args.insert("format".to_string(), Value::from(format));
            }
            if quality != 0.5 {
                call_args.insert("quality".to_string(), Value::from(quality));
            }
            dispatch_call("browser_record_start", call_args, headless, json_output).await;
        }
        Some(("stop", sub)) => {
            let output = sub.get_one::<String>("output").cloned().unwrap_or_default();
            let mut call_args = Map::new();
            if !output.is_empty() {
                call_args.insert("path".to_string(), Value::from(output));
            }
            dispatch_call("browser_record_stop", call_args, headless, json_output).await;
        }
        Some(("group", group_matches)) => match group_matches.subcommand() {
            Some(("start", sub)) => {
                let name = sub.get_one::<String>("name").cloned().unwrap_or_default();
                let mut call_args = Map::new();
                call_args.insert("name".to_string(), Value::from(name));
                dispatch_call("browser_record_start_group", call_args, headless, json_output).await;
            }
            Some(("stop", _)) => {
                dispatch_call("browser_record_stop_group", Map::new(), headless, json_output).await;
            }
            _ => print_subcommand_help("group"),
        },
        Some(("chunk", chunk_matches)) => match chunk_matches.subcommand() {
            Some(("start", sub)) => {
                let name = sub.get_one::<String>("name").cloned().unwrap_or_default();
                let title = sub.get_one::<String>("title").cloned().unwrap_or_default();
                let mut call_args = Map::new();
                if !name.is_empty() {
                    call_args.insert("name".to_string(), Value::from(name));
                }
                if !title.is_empty() {
                    call_args.insert("title".to_string(), Value::from(title));
                }
                dispatch_call("browser_record_start_chunk", call_args, headless, json_output).await;
            }
            Some(("stop", sub)) => {
                let output = sub.get_one::<String>("output").cloned().unwrap_or_default();
                let mut call_args = Map::new();
                if !output.is_empty() {
                    call_args.insert("path".to_string(), Value::from(output));
                }
                dispatch_call("browser_record_stop_chunk", call_args, headless, json_output).await;
            }
            _ => print_subcommand_help("chunk"),
        },
        // No subcommand: mirror Go's `cmd.Help()`.
        _ => {
            let _ = record_command().print_help();
            println!();
        }
    }
}

/// Sends a daemon tool call and prints the result (or error).
async fn dispatch_call(tool: &str, args: Map<String, Value>, headless: bool, json_output: bool) {
    match daemon_call(tool, args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}

/// Prints the help text for a `record` subcommand (mirrors cobra's `cmd.Help()`).
fn print_subcommand_help(name: &str) {
    let mut cmd = record_command();
    if let Some(sc) = cmd.find_subcommand_mut(name) {
        let _ = sc.print_help();
        println!();
    }
}
