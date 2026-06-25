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
#[cfg(test)]
mod tests;

use std::io::IsTerminal;
use std::io::Write as _;

use anstyle::{AnsiColor, Color, Style};
use clap::builder::Styles;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tokio_tungstenite::tungstenite::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};

/// browserlane's own version (from Cargo.toml).
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Branded colour scheme for clap's own help/usage/error rendering, matching the
/// dashboard's cyan accent. clap routes this through anstream, so it is stripped
/// automatically on a non-TTY stream and under `NO_COLOR`.
const HELP_STYLES: Styles = {
    let bold_cyan = Style::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    let cyan = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    Styles::styled()
        // Section headers ("Usage:", "Options:", "Commands:") and the usage line.
        .header(bold_cyan)
        .usage(bold_cyan)
        // The things a user types: subcommand names, flags, the program name.
        .literal(cyan)
        // Value placeholders (<URL>, [OPTIONS]) stay de-emphasised.
        .placeholder(Style::new().dimmed())
};


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
        // Pin the displayed binary name. clap otherwise derives it from argv0 at
        // runtime, so on case-insensitive filesystems a mis-cased invocation
        // (`BL`, `Bl`, …) would leak into clap's own usage/error strings. `prog`
        // is already normalized by prog_name(), keeping usage canonical.
        .bin_name(prog)
        .about("Browser automation for AI agents and humans")
        .version(ver)
        .styles(HELP_STYLES)
        // `--verbose` is the only true global: it drives central logging and is
        // honored by every command. `--headless` and `--json` are scoped
        // per-command (attached below via `apply_caps`) so each command's help
        // lists only the flags it actually consumes.
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Enable debug logging"),
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
        .subcommand(cmd::page_command())
        // Hidden, introspection-only: dumps the command tree as JSON for the
        // docs generator. Registered last so it sits at the end of the tree and
        // never perturbs the visible command order.
        .subcommand(cmd::dump_command());
    // ext-seam (browserlane extension hook)
    let cli = ext::register_cli(cli);

    // Scope `--headless`/`--json` per command: attach each flag only where the
    // command honors it (see `cmd::caps_for`). Done after ext registration so
    // extension commands are covered too, and recursively so nested subcommand
    // trees (daemon, cookies, find, …) get the flags at every level.
    let sub_names: Vec<String> = cli
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    let mut cli = cli;
    for name in sub_names {
        cli = cli.mut_subcommand(name, apply_caps);
    }
    cli
}

/// `--headless` as a per-command arg (not global), attached only to commands that
/// launch a browser — so it appears in *their* `--help` and clap rejects it on
/// commands that have no window to hide.
fn headless_arg() -> Arg {
    Arg::new("headless")
        .long("headless")
        .action(ArgAction::SetTrue)
        .help("Hide browser window (visible by default)")
}

/// `--json` as a per-command arg (not global), attached only to commands that emit
/// a structured result — so it advertises JSON only where the command honors it.
fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help("Output as JSON")
}

/// Attaches the per-command `--headless`/`--json` flags to `cmd` (and, recursively,
/// its nested subcommands) according to [`cmd::caps_for`]. The root keeps only the
/// true global (`--verbose`); these two are intentionally per-command so help
/// never lists a flag the command ignores.
fn apply_caps(mut cmd: Command) -> Command {
    let caps = cmd::caps_for(cmd.get_name());
    if caps.headless {
        cmd = cmd.arg(headless_arg());
    }
    if caps.json {
        cmd = cmd.arg(json_arg());
    }
    let sub_names: Vec<String> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    for name in sub_names {
        cmd = cmd.mut_subcommand(name, apply_caps);
    }
    cmd
}

/// Reads a global flag, falling back to the subcommand matches (clap stores
/// global args wherever they were parsed). Used for `--verbose`, the one flag
/// that is still global.
fn global_flag(root: &ArgMatches, sub: Option<&ArgMatches>, name: &str) -> bool {
    root.get_flag(name) || sub.map(|s| s.get_flag(name)).unwrap_or(false)
}

/// Reads a per-command bool flag (`--headless`/`--json`) at any depth of the
/// matched subcommand chain. Those flags are scoped per command (not global), and
/// clap records each on whichever command scope parsed it, so `bl daemon --json
/// status` and `bl daemon status --json` both register. `try_get_one` is the
/// non-panicking accessor: on a command that never defines the flag it returns an
/// error (treated as `false`) rather than panicking like `get_flag` would.
fn cmd_flag(matches: &ArgMatches, name: &str) -> bool {
    let mut m = matches;
    loop {
        if matches!(m.try_get_one::<bool>(name), Ok(Some(true))) {
            return true;
        }
        match m.subcommand() {
            Some((_, sub)) => m = sub,
            None => return false,
        }
    }
}

#[tokio::main]
async fn main() {
    // Help/version/error handling is now clap-native (the captured-cobra help
    // system was removed in the clap-help migration). One seam remains: the ROOT
    // help is a grouped, category-bucketed layout that clap cannot render itself,
    // so we intercept *only* the root `--help`/`-h` (no subcommand on the line)
    // and hand it to `render_root_help`. Per-command `<cmd> --help`, `--version`,
    // unknown-command "did you mean", and all argument errors flow through clap.
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let wants_help = raw_args.iter().any(|a| a == "--help" || a == "-h");
    if wants_help && !raw_args.iter().any(|a| !a.starts_with('-')) {
        // Root `--help` with no subcommand token: render the grouped layout from
        // the live command tree and exit 0. Written through `anstream::stdout()`
        // (like `print_dashboard` and clap's own help) so the branded ANSI is
        // stripped on a non-TTY stream and under `NO_COLOR` — piped `bl --help`
        // stays byte-for-byte plain.
        let mut out = anstream::stdout();
        let _ = write!(out, "{}", cmd::render_root_help(&build_cli()));
        std::process::exit(0);
    }

    // Let clap parse; it prints help/version (exit 0) and errors (its own exit
    // codes + native "did you mean" suggestions) directly via `Error::exit()`.
    let matches = match build_cli().try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };

    let sub = matches.subcommand().map(|(_, m)| m);

    // PersistentPreRun: enable logging only if --verbose is used.
    if global_flag(&matches, sub, "verbose") {
        log::setup(log::Level::Verbose);
    }
    let headless = cmd_flag(&matches, "headless");
    let json_output = cmd_flag(&matches, "json");

    match matches.subcommand() {
        // Hidden introspection command: emit the clap tree as JSON and return.
        // Build a fresh tree to hand the renderer the full root command.
        Some(("__dump", _)) => cmd::run_dump(&build_cli()),
        Some(("version", _)) => cmd::run_version(json_output),
        Some(("paths", _)) => cmd::run_paths(json_output),
        Some(("a11y-tree", sub)) => cmd::run_a11y_tree(sub, headless, json_output).await,
        Some(("is-installed", _)) => cmd::run_is_installed(json_output),
        Some(("install", _)) => cmd::run_install(json_output).await,
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
            // prints the completion command's help (clap-native) and exits 0,
            // mirroring cobra.
            if !cmd::run_completion(shell, build_cli()) {
                let mut cli = build_cli();
                if let Some(sc) = cli.find_subcommand_mut("completion") {
                    print!("{}", sc.render_help());
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
            // grouped root help so scripts and the smoke harness get a stable,
            // parseable command listing.
            if json_output || !std::io::stdout().is_terminal() {
                // Grouped root help, written through `anstream::stdout()` so its
                // branded ANSI is stripped on the non-TTY / `--json` path (and
                // honours `NO_COLOR`), exactly like `print_dashboard`.
                let mut out = anstream::stdout();
                let _ = write!(out, "{}", cmd::render_root_help(&build_cli()));
            } else {
                cmd::print_dashboard();
            }
        }
    }
}
