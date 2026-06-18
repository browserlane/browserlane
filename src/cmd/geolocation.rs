use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn geolocation_command() -> Command {
    Command::new("geolocation")
        .about("Override the browser geolocation")
        .arg(Arg::new("latitude").required(true).num_args(1))
        .arg(Arg::new("longitude").required(true).num_args(1))
        .arg(
            Arg::new("accuracy")
                .long("accuracy")
                .value_parser(clap::value_parser!(f64))
                .help("Accuracy in meters (default: 1)"),
        )
}

pub async fn run_geolocation(
    latitude: String,
    longitude: String,
    accuracy: f64,
    headless: bool,
    json_output: bool,
) {
    let lat: f64 = match latitude.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: invalid latitude: {latitude}");
            std::process::exit(1);
        }
    };
    let lng: f64 = match longitude.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: invalid longitude: {longitude}");
            std::process::exit(1);
        }
    };

    let mut call_args = Map::new();
    call_args.insert("latitude".to_string(), Value::from(lat));
    call_args.insert("longitude".to_string(), Value::from(lng));
    if accuracy > 0.0 {
        call_args.insert("accuracy".to_string(), Value::from(accuracy));
    }

    match daemon_call("browser_set_geolocation", call_args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
