use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn media_command() -> Command {
    Command::new("media")
        .about("Override CSS media features")
        .arg(
            Arg::new("color-scheme")
                .long("color-scheme")
                .help("Color scheme: light, dark, no-preference"),
        )
        .arg(
            Arg::new("reduced-motion")
                .long("reduced-motion")
                .help("Reduced motion: reduce, no-preference"),
        )
        .arg(
            Arg::new("forced-colors")
                .long("forced-colors")
                .help("Forced colors: active, none"),
        )
        .arg(
            Arg::new("contrast")
                .long("contrast")
                .help("Contrast: more, less, no-preference"),
        )
        .arg(
            Arg::new("media")
                .long("media")
                .help("Media type: screen, print"),
        )
}

pub async fn run_media(
    color_scheme: Option<String>,
    reduced_motion: Option<String>,
    forced_colors: Option<String>,
    contrast: Option<String>,
    media: Option<String>,
    headless: bool,
    json_output: bool,
) {
    let mut call_args = Map::new();
    if let Some(v) = color_scheme.filter(|s| !s.is_empty()) {
        call_args.insert("colorScheme".to_string(), Value::from(v));
    }
    if let Some(v) = reduced_motion.filter(|s| !s.is_empty()) {
        call_args.insert("reducedMotion".to_string(), Value::from(v));
    }
    if let Some(v) = forced_colors.filter(|s| !s.is_empty()) {
        call_args.insert("forcedColors".to_string(), Value::from(v));
    }
    if let Some(v) = contrast.filter(|s| !s.is_empty()) {
        call_args.insert("contrast".to_string(), Value::from(v));
    }
    if let Some(v) = media.filter(|s| !s.is_empty()) {
        call_args.insert("media".to_string(), Value::from(v));
    }

    if call_args.is_empty() {
        eprintln!("Error: at least one media feature flag is required");
        std::process::exit(1);
    }

    match daemon_call("browser_emulate_media", call_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
