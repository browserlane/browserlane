use clap::{Arg, Command};
use serde_json::{Map, Value};

use super::daemon_client::daemon_call;
use super::output::{print_error, print_result};

pub fn pdf_command() -> Command {
    Command::new("pdf")
        .about("Save page as PDF")
        .arg(Arg::new("url").num_args(0..=1))
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .default_value("page.pdf")
                .help("Output file path"),
        )
}

pub async fn run_pdf(url: Option<String>, output: String, headless: bool, json_output: bool) {
    if let Some(url) = url {
        let mut m = Map::new();
        m.insert("url".to_string(), Value::from(url));
        if let Err(e) = daemon_call("browser_navigate", m, headless).await {
            print_error(&e, json_output);
        }
    }

    let mut args = Map::new();
    args.insert("filename".to_string(), Value::from(output));
    match daemon_call("browser_pdf", args, headless).await {
        Ok(result) => print_result(&result, json_output),
        Err(e) => print_error(&e, json_output),
    }
}
