// Phase 1: the transport spine (bidi + browser) and the 7 diagnostic
// subcommands are wired. The remaining 62 commands are still header-only
// stubs and are registered in later phases.
#![allow(dead_code)]

mod agent;
mod api;
mod bidi;
mod browser;
mod cmd;
mod daemon;
mod errors;
mod ext;
mod log;
mod paths;
mod process;

use std::io::IsTerminal;

use clap::{Arg, ArgAction, ArgMatches, Command};
use tokio_tungstenite::tungstenite::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};

/// browserlane's own version (from Cargo.toml).
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");


/// Reads BROWSERLANE_CONNECT_URL and BROWSERLANE_CONNECT_API_KEY from the environment.
/// Returns the connect URL and any headers to send with the WebSocket connection.
pub(crate) fn connect_from_env() -> (String, Option<HeaderMap>) {
    let url = std::env::var("BROWSERLANE_CONNECT_URL").unwrap_or_default();
    let api_key = std::env::var("BROWSERLANE_CONNECT_API_KEY").unwrap_or_default();

    let headers = if !api_key.is_empty() {
        let mut h = HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            h.insert(AUTHORIZATION, value);
        }
        Some(h)
    } else {
        None
    };

    (url, headers)
}

/// Parses `--connect-header "K: V"` flags into a HeaderMap (pipe mode).
fn parse_connect_headers(values: &[String]) -> Option<HeaderMap> {
    if values.is_empty() {
        return None;
    }
    let mut headers = HeaderMap::new();
    for h in values {
        if let Some((k, v)) = h.split_once(':') {
            use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
            if let (Ok(name), Ok(val)) = (
                k.trim().parse::<HeaderName>(),
                v.trim().parse::<HeaderValue>(),
            ) {
                headers.append(name, val);
            }
        }
    }
    Some(headers)
}

/// Builds the CLI root, registering the Phase 1 subcommands in the same order
/// as main.go's `AddCommand` list.
fn build_cli() -> Command {
    // clap wants &'static str; the process is short-lived so leaking is fine.
    let prog: &'static str = Box::leak(cmd::prog_name().into_boxed_str());
    let ver: &'static str = Box::leak(format!("v{VERSION}").into_boxed_str());
    let cli = Command::new(prog)
        .about("Browser automation for AI agents and humans")
        .version(ver)
        .arg(
            Arg::new("headless")
                .long("headless")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Hide browser window (visible by default)"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Enable debug logging"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
        .subcommand(cmd::version_command())
        .subcommand(cmd::paths_command())
        .subcommand(cmd::a11y_tree_command())
        .subcommand(cmd::is_installed_command())
        .subcommand(cmd::install_command())
        .subcommand(cmd::launch_test_command())
        .subcommand(cmd::ws_test_command())
        .subcommand(cmd::bidi_test_command())
        .subcommand(cmd::navigate_command())
        .subcommand(cmd::screenshot_command())
        .subcommand(cmd::serve_command())
        .subcommand(cmd::pipe_command())
        .subcommand(cmd::completion_command())
        .subcommand(cmd::mcp_command())
        .subcommand(cmd::daemon_command())
        .subcommand(cmd::text_command())
        .subcommand(cmd::url_command())
        .subcommand(cmd::title_command())
        .subcommand(cmd::html_command())
        .subcommand(cmd::wait_command())
        .subcommand(cmd::back_command())
        .subcommand(cmd::forward_command())
        .subcommand(cmd::reload_command())
        .subcommand(cmd::start_command())
        .subcommand(cmd::stop_command())
        .subcommand(cmd::click_command())
        .subcommand(cmd::type_command())
        .subcommand(cmd::hover_command())
        .subcommand(cmd::select_command())
        .subcommand(cmd::scroll_command())
        .subcommand(cmd::keys_command())
        .subcommand(cmd::pages_command())
        .subcommand(cmd::fill_command())
        .subcommand(cmd::press_command())
        .subcommand(cmd::check_command())
        .subcommand(cmd::uncheck_command())
        .subcommand(cmd::dblclick_command())
        .subcommand(cmd::focus_command())
        .subcommand(cmd::cookies_command())
        .subcommand(cmd::dialog_command())
        .subcommand(cmd::download_command())
        .subcommand(cmd::frame_command())
        .subcommand(cmd::frames_command())
        .subcommand(cmd::upload_command())
        .subcommand(cmd::record_command())
        .subcommand(cmd::skill_command())
        .subcommand(cmd::map_command())
        .subcommand(cmd::diff_command())
        .subcommand(cmd::drag_command())
        .subcommand(cmd::viewport_command())
        .subcommand(cmd::window_command())
        .subcommand(cmd::value_command())
        .subcommand(cmd::attr_command())
        .subcommand(cmd::eval_command())
        .subcommand(cmd::find_command())
        .subcommand(cmd::count_command())
        .subcommand(cmd::is_command())
        .subcommand(cmd::mouse_command())
        .subcommand(cmd::storage_command())
        .subcommand(cmd::pdf_command())
        .subcommand(cmd::highlight_command())
        .subcommand(cmd::sleep_command())
        .subcommand(cmd::geolocation_command())
        .subcommand(cmd::content_command())
        .subcommand(cmd::media_command())
        .subcommand(cmd::page_command());
    // ext-seam (browserlane extension hook)
    ext::register_cli(cli)
}

/// Returns the space-joined path of the deepest selected subcommand chain
/// ("" if none), e.g. "mouse click" or "record group".
fn selected_command_path(matches: &ArgMatches) -> String {
    let mut parts = Vec::new();
    let mut m = matches;
    while let Some((name, sub)) = m.subcommand() {
        parts.push(name.to_string());
        m = sub;
    }
    parts.join(" ")
}

/// Reads a global flag, falling back to the subcommand matches (clap stores
/// global args wherever they were parsed).
fn global_flag(root: &ArgMatches, sub: Option<&ArgMatches>, name: &str) -> bool {
    root.get_flag(name) || sub.map(|s| s.get_flag(name)).unwrap_or(false)
}

/// Root subcommands whose name is "close" to `typed`, mirroring cobra's
/// `SuggestionsFor`: case-insensitive Levenshtein distance <= 2, OR the command
/// name starts with the typed string. Hidden commands (e.g. pipe/serve) are
/// excluded, and the order follows registration order.
fn command_suggestions(typed: &str) -> Vec<String> {
    let t = typed.to_lowercase();
    build_cli()
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| c.get_name().to_string())
        .filter(|name| {
            let n = name.to_lowercase();
            n.starts_with(&t) || levenshtein(&t, &n) <= 2
        })
        .collect()
}

/// Case-sensitive Levenshtein edit distance (callers lowercase first).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Handles a clap parse failure the way Go's cobra + `main()` do: `--help` and
/// `--version` print to stdout and exit 0; every other parse error exits with
/// code 1 (cobra), not clap's default 2. Unknown top-level commands are rendered
/// byte-for-byte like cobra: `Error: unknown command "X" for "bl"`, then the
/// usage hint, then the repeated message (mirroring `main()`'s
/// `fmt.Fprintln(os.Stderr, err)`).
fn handle_cli_parse_error(e: clap::Error) -> ! {
    use clap::error::{ContextKind, ContextValue, ErrorKind};

    match e.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            // clap writes help/version to stdout; cobra exits 0 for these.
            let _ = e.print();
            std::process::exit(0);
        }
        ErrorKind::InvalidSubcommand => {
            let prog = cmd::prog_name();
            let name = match e.get(ContextKind::InvalidSubcommand) {
                Some(ContextValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let msg = format!("unknown command {name:?} for {prog:?}");
            let suggestions = command_suggestions(&name);
            if suggestions.is_empty() {
                // cobra: `Error: <msg>` + usage hint, then main() re-prints err.
                eprintln!("Error: {msg}");
                eprintln!("Run '{prog} --help' for usage.");
                eprintln!("{msg}");
            } else {
                // cobra appends a "Did you mean this?" block (Levenshtein <= 2 or
                // prefix matches) to the error message, prints `Error: <err>` + the
                // usage hint, then main() re-prints the full err — so the suggestion
                // block appears twice. Reproduced byte-for-byte.
                let mut block = format!("{msg}\n\nDid you mean this?\n");
                for s in &suggestions {
                    block.push('\t');
                    block.push_str(s);
                    block.push('\n');
                }
                let out = format!("Error: {block}\nRun '{prog} --help' for usage.\n{block}\n");
                eprint!("{out}");
            }
            std::process::exit(1);
        }
        _ => {
            // Other parse errors keep clap's message for now (cobra-exact text +
            // the full per-command usage dump land with the --help format work),
            // but match cobra's exit code 1.
            let _ = e.print();
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    // cobra-faithful `--help`: serve the captured help text for the requested
    // command (with the live program name substituted) and exit 0, before clap
    // parses. clap's renderer cannot reproduce cobra's format, so we bypass it.
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(text) = cmd::intercept_help(&raw_args) {
        print!("{text}");
        std::process::exit(0);
    }

    let matches = match build_cli().try_get_matches() {
        Ok(m) => m,
        Err(e) => handle_cli_parse_error(e),
    };

    // A parent command whose cobra Run is cmd.Help() prints its help when invoked
    // with no subcommand. Serve the cobra-exact text instead of clap's renderer.
    let selected = selected_command_path(&matches);
    if cmd::shows_help_on_no_subcommand(&selected) {
        if let Some(text) = cmd::command_help(&selected) {
            print!("{text}");
            return;
        }
    }

    let sub = matches.subcommand().map(|(_, m)| m);

    // PersistentPreRun: enable logging only if --verbose is used.
    if global_flag(&matches, sub, "verbose") {
        log::setup(log::Level::Verbose);
    }
    let headless = global_flag(&matches, sub, "headless");
    let json_output = global_flag(&matches, sub, "json");

    match matches.subcommand() {
        Some(("version", _)) => cmd::run_version(),
        Some(("paths", _)) => cmd::run_paths(),
        Some(("a11y-tree", sub)) => cmd::run_a11y_tree(sub, headless, json_output).await,
        Some(("is-installed", _)) => cmd::run_is_installed(),
        Some(("install", _)) => cmd::run_install().await,
        Some(("launch-test", _)) => cmd::run_launch_test(headless).await,
        Some(("ws-test", sub)) => {
            let url = sub.get_one::<String>("url").cloned().unwrap_or_default();
            cmd::run_ws_test(url).await;
        }
        Some(("bidi-test", _)) => cmd::run_bidi_test().await,
        Some(("go", sub)) => {
            let url = sub.get_one::<String>("url").cloned().unwrap_or_default();
            cmd::run_navigate(url, headless, json_output).await;
        }
        Some(("screenshot", sub)) => {
            let url = sub.get_one::<String>("url").cloned();
            let output = sub.get_one::<String>("output").cloned().unwrap_or_else(|| "screenshot.png".to_string());
            let full_page = sub.get_flag("full-page");
            let annotate = sub.get_flag("annotate");
            cmd::run_screenshot(url, output, full_page, annotate, headless, json_output).await;
        }
        Some(("serve", sub)) => {
            let port = sub.get_one::<u16>("port").copied().unwrap_or(9515);
            cmd::run_serve(port, headless).await;
        }
        Some(("pipe", sub)) => {
            let connect_url = sub.get_one::<String>("connect").cloned().unwrap_or_default();
            let header_strs: Vec<String> = sub
                .get_many::<String>("connect-header")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let connect_headers = parse_connect_headers(&header_strs);
            cmd::run_pipe(connect_url, connect_headers, headless).await;
        }
        Some(("completion", sub)) => {
            let shell = sub.get_one::<String>("shell").map(String::as_str);
            // A valid shell emits the script (exit 0). A missing/unrecognized shell
            // prints the completion help and exits 0, mirroring cobra.
            if !cmd::run_completion(shell, build_cli()) {
                if let Some(text) = cmd::command_help("completion") {
                    print!("{text}");
                }
            }
        }
        Some(("mcp", sub)) => {
            let screenshot_dir = sub.get_one::<String>("screenshot-dir").cloned();
            cmd::run_mcp(screenshot_dir).await;
        }
        Some(("daemon", daemon_matches)) => {
            cmd::run_daemon(daemon_matches, headless, json_output).await;
        }
        Some(("text", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            cmd::run_text(args, headless, json_output).await;
        }
        Some(("html", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let outer = sub.get_flag("outer");
            cmd::run_html(args, outer, headless, json_output).await;
        }
        Some(("url", _)) => cmd::run_url(headless, json_output).await,
        Some(("title", _)) => cmd::run_title(headless, json_output).await,
        Some(("wait", wait_matches)) => cmd::run_wait(wait_matches, headless, json_output).await,
        Some(("back", _)) => cmd::run_back(headless, json_output).await,
        Some(("forward", _)) => cmd::run_forward(headless, json_output).await,
        Some(("reload", _)) => cmd::run_reload(headless, json_output).await,
        Some(("start", sub)) => {
            let url = sub.get_one::<String>("url").cloned();
            cmd::run_start(url, headless, json_output).await;
        }
        Some(("stop", _)) => cmd::run_stop(headless, json_output).await,
        Some(("pages", _)) => cmd::run_pages(headless, json_output).await,
        Some(("click", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let timeout = sub.get_one::<String>("timeout").cloned().unwrap_or_else(|| "30s".to_string());
            cmd::run_click(args, timeout, headless, json_output).await;
        }
        Some(("type", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let timeout = sub.get_one::<String>("timeout").cloned().unwrap_or_else(|| "30s".to_string());
            cmd::run_type(args, timeout, headless, json_output).await;
        }
        Some(("hover", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            cmd::run_hover(args, headless, json_output).await;
        }
        Some(("select", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            let value = sub.get_one::<String>("value").cloned().unwrap_or_default();
            cmd::run_select(selector, value, headless, json_output).await;
        }
        Some(("scroll", scroll_matches)) => cmd::run_scroll(scroll_matches, headless, json_output).await,
        Some(("keys", sub)) => {
            let keys = sub.get_one::<String>("keys").cloned().unwrap_or_default();
            cmd::run_keys(keys, headless, json_output).await;
        }
        Some(("fill", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            let text = sub.get_one::<String>("text").cloned().unwrap_or_default();
            cmd::run_fill(selector, text, headless, json_output).await;
        }
        Some(("press", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            cmd::run_press(args, headless, json_output).await;
        }
        Some(("check", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_check(selector, headless, json_output).await;
        }
        Some(("uncheck", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_uncheck(selector, headless, json_output).await;
        }
        Some(("dblclick", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_dblclick(selector, headless, json_output).await;
        }
        Some(("focus", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_focus(selector, headless, json_output).await;
        }
        Some(("drag", sub)) => {
            let source = sub.get_one::<String>("source").cloned().unwrap_or_default();
            let target = sub.get_one::<String>("target").cloned().unwrap_or_default();
            cmd::run_drag(source, target, headless, json_output).await;
        }
        Some(("cookies", cookies_matches)) => {
            cmd::run_cookies(cookies_matches, headless, json_output).await;
        }
        Some(("dialog", dialog_matches)) => {
            cmd::run_dialog(dialog_matches, headless, json_output).await;
        }
        Some(("download", download_matches)) => {
            cmd::run_download(download_matches, headless, json_output).await;
        }
        Some(("frame", frame_matches)) => {
            cmd::run_frame(frame_matches, headless, json_output).await;
        }
        Some(("frames", _)) => cmd::run_frames(headless, json_output).await,
        Some(("upload", upload_matches)) => {
            cmd::run_upload(upload_matches, headless, json_output).await;
        }
        Some(("record", record_matches)) => {
            cmd::run_record(record_matches, headless, json_output).await;
        }
        Some(("add-skill", skill_matches)) => cmd::run_skill(skill_matches),
        Some(("map", map_matches)) => cmd::run_map(map_matches, headless, json_output).await,
        Some(("diff", diff_matches)) => cmd::run_diff(diff_matches, headless, json_output).await,
        Some(("storage", storage_matches)) => {
            cmd::run_storage(storage_matches, headless, json_output).await;
        }
        Some(("viewport", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let dpr = sub.get_one::<f64>("dpr").copied().unwrap_or(0.0);
            cmd::run_viewport(args, dpr, headless, json_output).await;
        }
        Some(("window", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let state = sub.get_one::<String>("state").cloned();
            cmd::run_window(args, state, headless, json_output).await;
        }
        Some(("value", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_value(selector, headless, json_output).await;
        }
        Some(("attr", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            let attribute = sub.get_one::<String>("attribute").cloned().unwrap_or_default();
            cmd::run_attr(selector, attribute, headless, json_output).await;
        }
        Some(("eval", sub)) => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let use_stdin = sub.get_flag("stdin");
            cmd::run_eval(args, use_stdin, headless, json_output).await;
        }
        Some(("find", find_matches)) => cmd::run_find(find_matches, headless, json_output).await,
        Some(("count", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_count(selector, headless, json_output).await;
        }
        Some(("is", is_matches)) => cmd::run_is(is_matches, headless, json_output).await,
        Some(("mouse", mouse_matches)) => cmd::run_mouse(mouse_matches, headless, json_output).await,
        Some(("pdf", sub)) => {
            let url = sub.get_one::<String>("url").cloned();
            let output = sub.get_one::<String>("output").cloned().unwrap_or_else(|| "page.pdf".to_string());
            cmd::run_pdf(url, output, headless, json_output).await;
        }
        Some(("highlight", sub)) => {
            let selector = sub.get_one::<String>("selector").cloned().unwrap_or_default();
            cmd::run_highlight(selector, headless, json_output).await;
        }
        Some(("sleep", sub)) => {
            let ms = sub.get_one::<String>("ms").cloned().unwrap_or_default();
            cmd::run_sleep(ms, headless, json_output).await;
        }
        Some(("geolocation", sub)) => {
            let latitude = sub.get_one::<String>("latitude").cloned().unwrap_or_default();
            let longitude = sub.get_one::<String>("longitude").cloned().unwrap_or_default();
            let accuracy = sub.get_one::<f64>("accuracy").copied().unwrap_or(0.0);
            cmd::run_geolocation(latitude, longitude, accuracy, headless, json_output).await;
        }
        Some(("content", sub)) => {
            let html = sub.get_one::<String>("html").cloned();
            let use_stdin = sub.get_flag("stdin");
            cmd::run_content(html, use_stdin, headless, json_output).await;
        }
        Some(("media", sub)) => {
            cmd::run_media(
                sub.get_one::<String>("color-scheme").cloned(),
                sub.get_one::<String>("reduced-motion").cloned(),
                sub.get_one::<String>("forced-colors").cloned(),
                sub.get_one::<String>("contrast").cloned(),
                sub.get_one::<String>("media").cloned(),
                headless,
                json_output,
            )
            .await;
        }
        Some(("page", page_matches)) => cmd::run_page(page_matches, headless, json_output).await,
        // ext-seam (browserlane extension hook)
        Some((name, sub)) if ext::dispatch_cli(name, sub, headless, json_output).await => {}
        _ => {
            // No subcommand. On an interactive terminal, greet with the compact
            // branded launch screen; on a pipe/redirect or under --json, print the
            // plain help so scripts and the smoke harness see unchanged bytes.
            if json_output || !std::io::stdout().is_terminal() {
                print!("{}", cmd::root_help());
            } else {
                cmd::print_dashboard();
            }
        }
    }
}
