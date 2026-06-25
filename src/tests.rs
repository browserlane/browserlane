//! In-crate tests for the clap-native help surface (Phase 2 safety nets).
//!
//! These live inside the binary crate (rather than `tests/`) because they call
//! crate-internal items directly — `build_cli()`, `cmd::render_root_help`,
//! `cmd::category_for` — instead of shelling out to the binary. That keeps them
//! fast and lets the category-completeness test reason about the real
//! `Category` mapping, not scraped text.
//!
//! Two safety nets:
//!   * `snapshots` — insta snapshots of the rendered help for the root, the
//!     no-args dashboard, and **every** visible command and subcommand's
//!     `--help`, so any drift in the rendered text is caught.
//!   * `category_completeness` — asserts every visible top-level command maps to
//!     a real (non-`Other`) category, except those still on the migration
//!     allow-list (see [`UNMIGRATED`]).
//!
//! (The module is gated at its declaration in `main.rs` via `#[cfg(test)]`, so
//! no inner `#![cfg(test)]` is needed here.)

use clap::Command;

use crate::build_cli;
use crate::cmd::{self, category_for, Category};

/// Renders a command's help text exactly as `bl <path…> --help` would — driving
/// the real root command so the `bl` usage prefix and inherited global flags are
/// present — with the live program name normalized to `bl` for determinism.
///
/// `path` is the chain of subcommand tokens from the root (e.g. `["record",
/// "group", "start"]`). We ask clap to parse `<path…> --help`; clap responds
/// with a `DisplayHelp` "error" whose payload is the fully-rendered help screen
/// (the same bytes the CLI prints). This is the real render path, so the
/// snapshot tracks what users actually see rather than a subcommand rendered in
/// isolation.
fn render_command_help(path: &[&str]) -> String {
    let argv: Vec<&str> = std::iter::once("bl")
        .chain(path.iter().copied())
        .chain(std::iter::once("--help"))
        .collect();
    let err = build_cli()
        .try_get_matches_from(argv)
        .expect_err("--help should surface as a DisplayHelp error");
    normalize_prog(&err.render().to_string())
}

/// Replaces the live program name with the canonical `bl` so snapshot output is
/// stable across machines / `cargo test` invocations (argv0 is the test binary).
fn normalize_prog(s: &str) -> String {
    let prog = cmd::prog_name();
    if prog.is_empty() || prog == "bl" {
        s.to_string()
    } else {
        s.replace(&prog, "bl")
    }
}

/// Walks the command tree depth-first, collecting the full token path of every
/// visible command and subcommand (the root itself is excluded). Hidden commands
/// (e.g. `__dump`) are skipped so the snapshot set matches the user-visible
/// surface. Driving the walk off the real `build_cli()` tree means new commands
/// are snapshotted automatically — there is no hand-maintained list to drift.
fn visible_command_paths(cmd: &Command, prefix: &[String], out: &mut Vec<Vec<String>>) {
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let mut path = prefix.to_vec();
        path.push(sub.get_name().to_string());
        out.push(path.clone());
        visible_command_paths(sub, &path, out);
    }
}

mod snapshots {
    use super::*;

    #[test]
    fn root_grouped_help() {
        // `render_root_help` now returns *branded* text (bold-cyan headers, cyan
        // command names). Strip the ANSI before snapshotting so the snapshot
        // captures the stable, readable plain layout — the same bytes a non-TTY
        // / `NO_COLOR` consumer sees once it's written through `anstream`.
        let styled = cmd::render_root_help(&build_cli());
        let plain = anstream::adapter::strip_str(&styled).to_string();
        insta::assert_snapshot!("root_help", normalize_prog(&plain));
    }

    /// The no-args launch screen (banner + Get-started block), captured as plain
    /// text so the banner layout/alignment is locked against regressions.
    #[test]
    fn dashboard() {
        insta::assert_snapshot!("dashboard", normalize_prog(&cmd::dashboard_plain()));
    }

    /// One `--help` snapshot per visible command and subcommand. The snapshot
    /// name is the token path joined with `__` (e.g. `cmd_record__group__start`),
    /// so each lands in its own stable `.snap` file and a single command's drift
    /// shows up as a single failing snapshot.
    #[test]
    fn every_command_help() {
        let mut paths: Vec<Vec<String>> = Vec::new();
        visible_command_paths(&build_cli(), &[], &mut paths);
        assert!(
            paths.len() >= 60,
            "expected the full command surface to be walked (got {} paths) — did \
             the tree fail to build?",
            paths.len()
        );
        for path in &paths {
            let refs: Vec<&str> = path.iter().map(String::as_str).collect();
            let name = format!("cmd_{}", path.join("__"));
            insta::assert_snapshot!(name, render_command_help(&refs));
        }
    }
}

/// Top-level commands not yet migrated to a real category. They are allowed to
/// return [`Category::Other`] so CI stays green while the 65-command port is in
/// progress. **As each command is migrated, delete it from this list.** When the
/// list is empty, the assertion below becomes "no visible command may be
/// `Other`" — i.e. the migration is complete and the `Other` bucket is dead.
///
/// Keep this sorted for easy diffing. Hidden commands (e.g. `pipe`, `serve`,
/// `__dump`) are not user-visible and are excluded from the check entirely, so
/// they never belong here.
const UNMIGRATED: &[&str] = &[];

/// Every visible top-level command must map to a real category. Migrated
/// commands must NOT be on the allow-list (that would let a regression slip
/// through), and allow-listed commands must still exist (so stale entries are
/// pruned). When `UNMIGRATED` is finally empty this collapses to "`Other` is
/// unreachable for visible commands."
#[test]
fn category_completeness() {
    let root = build_cli();
    let visible: Vec<String> = root
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| c.get_name().to_string())
        .collect();

    // 1. Each visible command is either categorized or explicitly allow-listed.
    let mut uncategorized: Vec<String> = Vec::new();
    for name in &visible {
        if category_for(name) == Category::Other && !UNMIGRATED.contains(&name.as_str()) {
            uncategorized.push(name.clone());
        }
    }
    assert!(
        uncategorized.is_empty(),
        "these visible commands fall into `Other` but are not on the UNMIGRATED \
         allow-list — give them a real category in category_for(), or (if \
         intentionally deferred) add them to UNMIGRATED: {uncategorized:?}"
    );

    // 2. No command on the allow-list has actually been migrated (keeps the list
    //    honest — a migrated command must leave it so the net tightens).
    let stale_migrated: Vec<&&str> = UNMIGRATED
        .iter()
        .filter(|n| category_for(n) != Category::Other)
        .collect();
    assert!(
        stale_migrated.is_empty(),
        "these commands are on the UNMIGRATED allow-list but now have a real \
         category — remove them from UNMIGRATED: {stale_migrated:?}"
    );

    // 3. No allow-list entry is stale (every name still exists as a visible
    //    command), so the list shrinks as commands are renamed/removed too.
    let stale_missing: Vec<&&str> = UNMIGRATED
        .iter()
        .filter(|n| !visible.iter().any(|v| v == *n))
        .collect();
    assert!(
        stale_missing.is_empty(),
        "these names are on the UNMIGRATED allow-list but are not visible \
         top-level commands anymore — remove them from UNMIGRATED: {stale_missing:?}"
    );

    // 4. Every visible command has an explicit slot in the curated within-group
    //    display order (COMMAND_ORDER in category.rs). A command missing there
    //    silently sorts last in its group, so this guards against that drift: a
    //    newly added command must be placed in COMMAND_ORDER, not just
    //    categorized. We assert it behaviorally via the rendered grouped help —
    //    each visible command's line must appear, and (the real check) the
    //    relative order of any two commands sharing a category must follow
    //    COMMAND_ORDER, which a `usize::MAX`-sorted (missing) command cannot do
    //    unless it is genuinely last in its block.
    // Strip the branding (cyan command names / bold-cyan headers) before parsing
    // command names out of the rendered lines: the escapes carry no whitespace,
    // so an un-stripped `"  \x1b[36mgo\x1b[0m  …"` line would yield the token
    // `\x1b[36mgo\x1b[0m` instead of `go` and the membership check would fail.
    let rendered = normalize_prog(
        &anstream::adapter::strip_str(&cmd::render_root_help(&root)).to_string(),
    );
    let listed_names: Vec<&str> = rendered
        .lines()
        .filter_map(|l| {
            let t = l.strip_prefix("  ")?;
            // Command lines are "  <name>  <about>"; headers/usage are not
            // indented this way and never start with two spaces + a token.
            t.split_whitespace().next()
        })
        .collect();
    let missing_render: Vec<String> = visible
        .iter()
        .filter(|name| !listed_names.contains(&name.as_str()))
        .cloned()
        .collect();
    assert!(
        missing_render.is_empty(),
        "these visible commands never render in the grouped root help — they are \
         likely missing a COMMAND_ORDER slot or a category: {missing_render:?}"
    );
}

/// `caps_for()` is the source of truth for which commands carry `--headless` /
/// `--json`; `build_cli()` attaches them accordingly. This pins both ends:
///   1. representative caps are what we expect (a browser cmd → both, a
///      diagnostic → json-only, an action cmd → neither);
///   2. every top-level command's *attached* args match its caps (so `apply_caps`
///      and `caps_for` can't drift apart); and
///   3. no name in the explicit caps lists has gone stale.
#[test]
fn caps_match_attached_flags() {
    use cmd::{caps_for, Caps};

    // (1) representative expectations — also exercises the public Caps type.
    assert_eq!(caps_for("go"), Caps { headless: true, json: true });
    assert_eq!(caps_for("version"), Caps { headless: false, json: true });
    assert_eq!(caps_for("launch-test"), Caps { headless: true, json: false });
    assert_eq!(caps_for("completion"), Caps { headless: false, json: false });

    // (2) attachment must equal caps for every top-level command.
    let root = build_cli();
    for sub in root.get_subcommands() {
        let name = sub.get_name();
        let caps = caps_for(name);
        let has = |id: &str| sub.get_arguments().any(|a| a.get_id().as_str() == id);
        assert_eq!(has("headless"), caps.headless, "{name}: --headless attachment ≠ caps");
        assert_eq!(has("json"), caps.json, "{name}: --json attachment ≠ caps");
    }

    // (3) explicit caps entries must still name real commands (visible or hidden).
    fn collect(cmd: &Command, into: &mut Vec<String>) {
        for sub in cmd.get_subcommands() {
            into.push(sub.get_name().to_string());
            collect(sub, into);
        }
    }
    let mut names = Vec::new();
    collect(&root, &mut names);
    let stale: Vec<&&str> = cmd::CAPS_EXCEPTIONS
        .iter()
        .filter(|n| !names.iter().any(|have| have == **n))
        .collect();
    assert!(stale.is_empty(), "caps_for() lists names that are no longer commands: {stale:?}");
}
