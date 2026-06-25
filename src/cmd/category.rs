//! clap-help migration: category tagging + grouped root-help.
//!
//! clap cannot group subcommands under headers natively, so the root `--help`
//! buckets `root.get_subcommands()` by a [`Category`] derived from each
//! command's name. Everything in this file is derived from the live clap
//! `Command` tree at render time — **the only hand-maintained data is the
//! category bucketing** (`category_for`) and the header order
//! (`CATEGORY_ORDER`). The about line, the usage line, the per-command
//! descriptions, the within-group order, and the global-flags block are all
//! pulled from the real `Command`, so changing `build_cli()` (adding a command,
//! a global flag, renaming, reordering) is automatically reflected here.
//!
//! Phase 2 scope: the three migrated commands (`back`, `screenshot`, `find`)
//! carry a real category; everything else falls into `Other`, printed under a
//! temporary "Other:" header. Finishing the migration (Phase 3) is just filling
//! in `category_for` arm-by-arm; when nothing returns `Other`, the variant and
//! header go away.

use clap::{Arg, Command};

use super::style::{BRAND, COMMAND};

/// The category buckets shown as headers in the grouped root `--help`, in the
/// exact order cobra emitted them. `Other` is the catch-all for the not-yet-
/// migrated commands (removed once `category_for` is complete).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Navigation,
    Interaction,
    InspectPage,
    Capture,
    BrowserState,
    Scripting,
    SessionDaemon,
    AgentMcp,
    SetupDiagnostics,
    Other,
}

impl Category {
    /// The header line printed above this group (matches the captured help).
    pub fn header(self) -> &'static str {
        match self {
            Category::Navigation => "Navigation:",
            Category::Interaction => "Interaction:",
            Category::InspectPage => "Inspect page:",
            Category::Capture => "Capture:",
            Category::BrowserState => "Browser state & emulation:",
            Category::Scripting => "Scripting:",
            Category::SessionDaemon => "Session & daemon:",
            Category::AgentMcp => "Agent & MCP:",
            Category::SetupDiagnostics => "Setup & diagnostics:",
            // Temporary bucket for un-migrated commands. Once every command has
            // a real category this arm (and the variant) go away.
            Category::Other => "Other:",
        }
    }
}

/// Header render order. Iterated by the root renderer so groups always appear
/// in this sequence regardless of subcommand registration order.
const CATEGORY_ORDER: &[Category] = &[
    Category::Navigation,
    Category::Interaction,
    Category::InspectPage,
    Category::Capture,
    Category::BrowserState,
    Category::Scripting,
    Category::SessionDaemon,
    Category::AgentMcp,
    Category::SetupDiagnostics,
    Category::Other,
];

/// Curated within-group display order for every top-level command, grouped by
/// category in `CATEGORY_ORDER` sequence. This is the second hand-maintained
/// seam (alongside `category_for`): clap yields subcommands in `build_cli()`
/// registration order, which does **not** match the curated per-header layout of
/// the captured cobra root, so the root renderer sorts each group by each
/// command's index here instead of by registration position.
///
/// The order within each block reproduces the captured `bl --help` exactly.
/// `add-mcp` is an ext-seam addition absent from that capture, slotted at the end
/// of its group (Agent & MCP). A command missing here sorts last in its group
/// (index `usize::MAX`); the `category_completeness` test guards against that
/// drift.
const COMMAND_ORDER: &[&str] = &[
    // Navigation
    "go",
    "back",
    "forward",
    "reload",
    "wait",
    // Interaction
    "click",
    "dblclick",
    "hover",
    "type",
    "fill",
    "press",
    "keys",
    "select",
    "check",
    "uncheck",
    "focus",
    "scroll",
    "drag",
    "mouse",
    "upload",
    // Inspect page
    "url",
    "title",
    "text",
    "html",
    "attr",
    "value",
    "count",
    "find",
    "map",
    "a11y-tree",
    "is",
    "pages",
    "frames",
    "frame",
    // Capture
    "screenshot",
    "pdf",
    "record",
    "highlight",
    // Browser state & emulation
    "cookies",
    "storage",
    "download",
    "dialog",
    "content",
    "diff",
    "viewport",
    "window",
    "media",
    "geolocation",
    // Scripting
    "eval",
    "sleep",
    // Session & daemon
    "start",
    "stop",
    "page",
    "daemon",
    // Agent & MCP
    "mcp",
    "add-skill",
    "add-mcp",
    // Setup & diagnostics
    "install",
    "is-installed",
    "paths",
    "version",
    "completion",
    "launch-test",
    "bidi-test",
    "ws-test",
];

/// The curated display index of a top-level command name within
/// [`COMMAND_ORDER`]. Unknown names sort last (`usize::MAX`) so a newly added,
/// not-yet-ordered command never reorders the rest of its group (the
/// `category_completeness` test flags the omission).
fn command_display_index(name: &str) -> usize {
    COMMAND_ORDER
        .iter()
        .position(|&c| c == name)
        .unwrap_or(usize::MAX)
}

/// Maps a top-level command *name* to its category. This is the single tagging
/// seam: it is keyed off `Command::get_name()`, so it is derived from the
/// registered command, not a duplicated command list.
///
/// Every visible top-level command carries a real category; `Other` is now only
/// a defensive fall-through for a name that has not been tagged (e.g. a newly
/// added command). The category-completeness test (`tests.rs`) guards this: a
/// visible command that returns `Other` and is not on the (now-empty) migration
/// allow-list fails the build, forcing a category to be assigned here.
pub fn category_for(name: &str) -> Category {
    match name {
        // Navigation
        "go" => Category::Navigation,
        "back" => Category::Navigation,
        "forward" => Category::Navigation,
        "reload" => Category::Navigation,
        "wait" => Category::Navigation,

        // Interaction
        "click" => Category::Interaction,
        "dblclick" => Category::Interaction,
        "hover" => Category::Interaction,
        "type" => Category::Interaction,
        "fill" => Category::Interaction,
        "press" => Category::Interaction,
        "keys" => Category::Interaction,
        "select" => Category::Interaction,
        "check" => Category::Interaction,
        "uncheck" => Category::Interaction,
        "focus" => Category::Interaction,
        "scroll" => Category::Interaction,
        "drag" => Category::Interaction,
        "mouse" => Category::Interaction,
        "upload" => Category::Interaction,

        // Inspect page
        "url" => Category::InspectPage,
        "title" => Category::InspectPage,
        "text" => Category::InspectPage,
        "html" => Category::InspectPage,
        "attr" => Category::InspectPage,
        "value" => Category::InspectPage,
        "count" => Category::InspectPage,
        "map" => Category::InspectPage,
        "a11y-tree" => Category::InspectPage,
        "is" => Category::InspectPage,
        "pages" => Category::InspectPage,
        "frames" => Category::InspectPage,
        "frame" => Category::InspectPage,
        "find" => Category::InspectPage,

        // Capture
        "screenshot" => Category::Capture,
        "pdf" => Category::Capture,
        "record" => Category::Capture,
        "highlight" => Category::Capture,

        // Browser state & emulation
        "cookies" => Category::BrowserState,
        "storage" => Category::BrowserState,
        "download" => Category::BrowserState,
        "dialog" => Category::BrowserState,
        "content" => Category::BrowserState,
        "diff" => Category::BrowserState,
        "viewport" => Category::BrowserState,
        "window" => Category::BrowserState,
        "media" => Category::BrowserState,
        "geolocation" => Category::BrowserState,

        // Scripting
        "eval" => Category::Scripting,
        "sleep" => Category::Scripting,

        // Session & daemon
        "start" => Category::SessionDaemon,
        "stop" => Category::SessionDaemon,
        "page" => Category::SessionDaemon,
        "daemon" => Category::SessionDaemon,

        // Setup & diagnostics
        "install" => Category::SetupDiagnostics,
        "is-installed" => Category::SetupDiagnostics,
        "paths" => Category::SetupDiagnostics,
        "version" => Category::SetupDiagnostics,
        "launch-test" => Category::SetupDiagnostics,
        "bidi-test" => Category::SetupDiagnostics,
        "ws-test" => Category::SetupDiagnostics,
        "completion" => Category::SetupDiagnostics,

        // Agent & MCP
        "mcp" => Category::AgentMcp,
        "add-skill" => Category::AgentMcp,
        "add-mcp" => Category::AgentMcp,

        _ => Category::Other,
    }
}

/// Which of the per-command flags (`--headless`, `--json`) a command actually
/// honors. This is the single source of truth for flag *scoping*: `build_cli()`
/// attaches `--headless`/`--json` to a command only when its caps say so, so each
/// command's `--help` advertises exactly the flags it consumes and clap rejects
/// the rest (`bl completion --json` errors instead of silently ignoring it).
///
/// `--verbose` is deliberately absent — it stays a true global (it drives central
/// logging and is honored everywhere), so it is not scoped here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Caps {
    pub headless: bool,
    pub json: bool,
}

/// Maps a command name to the flags it honors. Most commands drive the browser
/// and emit a result, so they honor BOTH — that is the default, and the match
/// lists only the exceptions. Nested subcommands inherit their parent's caps via
/// the recursive attachment in `build_cli()`; every nested leaf is itself a
/// browser op, so the `_ => both` default is correct for them too.
///
/// Adding a command: a browser command needs no entry (it defaults to both); a
/// new diagnostic / setup / agent / transport command that does NOT honor one of
/// the flags must be listed here, or its help will over-advertise. The
/// `caps_exceptions_exist` test keeps these lists free of stale names.
pub fn caps_for(name: &str) -> Caps {
    match name {
        // Query/diagnostic commands: structured result, but no browser window.
        "version" | "paths" | "is-installed" | "install" => Caps {
            headless: false,
            json: true,
        },
        // Launch a browser / transport, but emit no JSON result object.
        "launch-test" | "serve" | "pipe" => Caps {
            headless: true,
            json: false,
        },
        // No browser and no JSON result: shell script, long-running server,
        // interactive transport, or installer.
        "ws-test" | "bidi-test" | "completion" | "mcp" | "add-skill" | "add-mcp" | "__dump" => {
            Caps {
                headless: false,
                json: false,
            }
        }
        // Everything else drives the browser and emits a result → both.
        _ => Caps {
            headless: true,
            json: true,
        },
    }
}

/// The command names explicitly listed in [`caps_for`] (i.e. every non-default
/// command). Used by a test to assert none have gone stale.
#[cfg(test)]
pub const CAPS_EXCEPTIONS: &[&str] = &[
    "version",
    "paths",
    "is-installed",
    "install",
    "launch-test",
    "serve",
    "pipe",
    "ws-test",
    "bidi-test",
    "completion",
    "mcp",
    "add-skill",
    "add-mcp",
    "__dump",
];

/// Renders the root `--help` with subcommands grouped under category headers.
///
/// Everything except the category bucketing is derived from `root`:
///   * the about line — `root.get_about()`;
///   * the `Usage:` line — clap's native `render_usage()`;
///   * each command's one-line description — `Command::get_about()`;
///   * the within-group order — the curated sequence in [`COMMAND_ORDER`] (which
///     reproduces the captured cobra root's per-header layout); clap's own
///     `get_subcommands()` registration order is *not* used for ordering, only
///     to enumerate which commands exist;
///   * the global-flags block — clap-native, rendered from the root's global
///     args (see [`render_global_flags`]).
///
/// Branding: the section headers (`Usage:`, `Commands:`, each category
/// header, and `Options:`) render in **bold cyan** ([`BRAND`]) and the command
/// names in **cyan** ([`COMMAND`]) to match clap's branded per-command help
/// (`HELP_STYLES` in `main.rs`); descriptions, usage args, and flag descriptions
/// stay plain. The styled string is written through `anstream::stdout()` by the
/// caller, so the ANSI is stripped automatically on a non-TTY stream and under
/// `NO_COLOR` (and tests snapshot the stripped text).
pub fn render_root_help(root: &Command) -> String {
    let prog = root.get_name();
    let mut out = String::new();

    // About line (clap owns the text via `.about(...)`).
    if let Some(about) = root.get_about() {
        out.push_str(&about.to_string());
        out.push_str("\n\n");
    }

    // Usage line, rendered clap-native from the root command (so flags/args/
    // subcommand presence are reflected automatically). `render_usage` needs a
    // built `&mut Command`, so we clone the root rather than mutate the caller's.
    // Only the `Usage:` label is branded (bold cyan); the args stay plain, like
    // clap's own usage line.
    let mut root_for_usage = root.clone();
    let usage = root_for_usage.render_usage().to_string();
    let usage = usage.trim_end();
    match usage.split_once(' ') {
        Some((label, rest)) => {
            out.push_str(&format!("{BRAND}{label}{BRAND:#} {rest}"));
        }
        None => out.push_str(usage),
    }
    out.push_str("\n\n");

    out.push_str(&format!("{BRAND}Commands:{BRAND:#}\n"));

    // Bucket every visible subcommand by category. We only need the command set
    // here (not clap's registration order): the within-group sequence is the
    // curated COMMAND_ORDER, applied per group below.
    let visible: Vec<&Command> = root
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .collect();
    let name_width = visible
        .iter()
        .map(|c| c.get_name().len())
        .max()
        .unwrap_or(0);

    for &cat in CATEGORY_ORDER {
        let mut in_group: Vec<&Command> = visible
            .iter()
            .copied()
            .filter(|c| category_for(c.get_name()) == cat)
            .collect();
        if in_group.is_empty() {
            continue;
        }
        // Within a group, emit commands in curated COMMAND_ORDER, which matches
        // cobra's per-header layout. The sort key is each command's explicit
        // index in COMMAND_ORDER, so this is independent of however clap chooses
        // to iterate — never alphabetical, never registration-order.
        in_group.sort_by_key(|c| command_display_index(c.get_name()));

        out.push('\n');
        out.push_str(&format!("{BRAND}{}{BRAND:#}\n", cat.header()));
        for c in in_group {
            let name = c.get_name();
            let about = c.get_about().map(|s| s.to_string()).unwrap_or_default();
            // Pad to the column width *before* styling: ANSI escapes have zero
            // display width, so styling the padded cell would misalign the
            // descriptions. We style only the name, then pad the remainder.
            let pad = " ".repeat(name_width.saturating_sub(name.len()));
            out.push_str(&format!(
                "  {COMMAND}{name}{COMMAND:#}{pad}  {about}\n"
            ));
        }
    }

    // Global-flags block, derived clap-native from the root's global args.
    out.push('\n');
    out.push_str(&render_global_flags(root));

    out.push_str(&format!(
        "\nUse \"{prog} [command] --help\" for more information about a command.\n"
    ));

    out
}

/// Renders the `Options:` block for the grouped root help from the root's
/// *global* args, clap-native. We build a throwaway `Command` carrying just those
/// args (plus clap's auto-injected `-h/--help` and, since the root has a version,
/// `-V/--version`) and let clap render + column-align them exactly as it would
/// in any other help screen. The option lines are then extracted from clap's own
/// `Options:` section and re-emitted under a branded `Options:` header, so the
/// grouped root help matches clap-native per-command help.
///
/// Nothing here is hardcoded: adding `.arg(...global...)` in `build_cli()`
/// surfaces the flag in this block automatically, in clap's own format.
fn render_global_flags(root: &Command) -> String {
    let prog = root.get_name();
    // clap's Str wants a 'static name; the process is short-lived so leaking the
    // (already-normalized) program name is fine and matches build_cli().
    let prog_static: &'static str = Box::leak(prog.to_string().into_boxed_str());

    let mut synth = Command::new(prog_static)
        .bin_name(prog_static)
        // No usage/help-of-its-own should leak in; we only read the options list.
        .disable_help_subcommand(true);
    if let Some(ver) = root.get_version() {
        let ver_static: &'static str = Box::leak(ver.to_string().into_boxed_str());
        synth = synth.version(ver_static);
    }
    for arg in root.get_arguments().filter(|a| a.is_global_set()) {
        synth = synth.arg((*arg).clone());
    }

    let rendered = synth.render_help().to_string();
    let options = extract_options_block(&rendered);

    // `Options:` header in bold cyan to match the category headers / `Usage:`
    // (and clap-native per-command help); the option lines stay as clap renders
    // them (plain), matching the task's "flag descriptions left plain" rule.
    let mut out = format!("{BRAND}Options:{BRAND:#}\n");
    out.push_str(&options);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Pulls the body of the `Options:` section out of a clap-rendered help string:
/// every line after the `Options:` header up to the next blank line (clap groups
/// args under that single heading and terminates the section with a blank line).
/// Returns the option lines joined with newlines (trailing newline included).
fn extract_options_block(rendered: &str) -> String {
    let mut lines = rendered.lines();
    // Advance to the Options: header.
    for line in lines.by_ref() {
        if line.trim_end() == "Options:" {
            break;
        }
    }
    let mut body = String::new();
    for line in lines {
        if line.trim().is_empty() {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    body
}

/// Test/introspection helper: whether `name` has an explicit slot in the curated
/// [`COMMAND_ORDER`]. The `category_completeness` test uses this as a drift guard
/// so a newly added visible command must be given a display position (otherwise
/// it would silently sort last in its group).
#[allow(dead_code)]
pub fn is_in_command_order(name: &str) -> bool {
    COMMAND_ORDER.contains(&name)
}

/// Test/introspection helper: the registration-order index of a top-level
/// command name within `root` (the position clap yields it from
/// `get_subcommands()`, i.e. its `.subcommand()` order in `build_cli()`).
/// Returns `usize::MAX` for an unknown name so callers can sort unknowns last.
#[allow(dead_code)]
pub fn command_order(root: &Command, name: &str) -> usize {
    root.get_subcommands()
        .position(|c| c.get_name() == name)
        .unwrap_or(usize::MAX)
}

/// Returns the (visible) global args of `root`, in registration order. Exposed
/// for tests/introspection so the derived flags block can be asserted against
/// the real command surface.
#[allow(dead_code)]
pub fn global_args(root: &Command) -> Vec<&Arg> {
    root.get_arguments().filter(|a| a.is_global_set()).collect()
}
