//! Shared helper for building a command's `Examples:` help block (clap's
//! `after_help`) from structured data, with the **live program name** spliced
//! in.
//!
//! This is the template every migrated command copies, so the ergonomics
//! matter. A command lists its examples as `(snippet, comment)` pairs:
//!
//! ```ignore
//! Command::new("screenshot")
//!     .about("Capture a screenshot (optionally navigate to URL first)")
//!     // ...args...
//!     .after_help(examples(&[
//!         ("screenshot -o shot.png", "Screenshots the current page"),
//!         ("screenshot https://example.com -o shot.png",
//!          "Navigates to URL first, then screenshots"),
//!         ("screenshot -o full.png --full-page",
//!          "Capture the entire page (not just the viewport)"),
//!     ]))
//! ```
//!
//! and renders as
//!
//! ```text
//! Examples:
//!   bl screenshot -o shot.png
//!   # Screenshots the current page
//!
//!   bl screenshot https://example.com -o shot.png
//!   # Navigates to URL first, then screenshots
//! ```
//!
//! The program name comes from [`prog_name`] at build time, so examples follow
//! a renamed/symlinked binary (`xyz screenshot ...`) instead of the hardcoded
//! `bl` the spike emitted. A snippet whose first line already contains the
//! `{prog}` token (e.g. a shell pipeline like `echo ... | {prog} content
//! --stdin`) is rendered verbatim instead of getting the program prepended —
//! see [`examples`].

use super::diagnostics::prog_name;

/// Token callers may use anywhere in a snippet or comment to stand in for the
/// program name.
///
/// There are two ways a snippet uses it:
///   * **Continuation / comment mention** — `{prog}` appears somewhere *after*
///     the first line (e.g. a multi-step workflow, or inside the comment). The
///     first line still gets the program auto-prepended, so the program name on
///     line 1 is implicit and only the inner references need the token. This is
///     the common case (`map`, `diff`).
///   * **Self-describing first line** — `{prog}` appears on the snippet's
///     *first* line, which means the snippet already spells out the full command
///     line itself (e.g. a shell pipeline `echo ... | {prog} content --stdin`).
///     The helper then renders that first line verbatim and does **not**
///     auto-prepend the program.
pub const PROG: &str = "{prog}";

/// Builds a command's `after_help` "Examples:" block from `(snippet, comment)`
/// pairs, splicing in the live program name.
///
/// For each pair:
///   * `snippet` is normally the command **without** the leading program name;
///     the helper renders it as `  <prog> <snippet>`. (Pass an empty `snippet`
///     to emit a bare `  <prog>` line — e.g. a no-arg command like `back`.)
///     *Exception:* if the snippet's **first line** already contains the
///     `{prog}` token ([`PROG`]) it is treated as self-describing — the full
///     line is rendered verbatim (with `{prog}` substituted) and the program is
///     **not** auto-prepended. This lets a snippet express a shell pipeline such
///     as `echo '<h1>Hi</h1>' | {prog} content --stdin`, where the program is
///     not the first word on the line.
///   * `comment` is the explanation; the helper renders it as `  # <comment>`.
///     Pass `""` to omit the comment line entirely.
///
/// Examples are separated by a blank line, matching the captured cobra layout.
/// The `{prog}` token ([`PROG`]) is substituted anywhere it appears in either
/// field, so it also works for an inline program mention in a continuation line
/// or comment.
pub fn examples(pairs: &[(&str, &str)]) -> String {
    let prog = prog_name();
    let mut out = String::from("Examples:");
    for (i, (snippet, comment)) in pairs.iter().enumerate() {
        if i > 0 {
            // Blank line between consecutive examples.
            out.push('\n');
        }
        out.push_str("\n  ");
        // Decide whether to auto-prepend the program. A snippet whose *first*
        // line already carries `{prog}` spells out its own full command line
        // (e.g. a shell pipeline `echo ... | {prog} ...`), so it is rendered
        // verbatim. Otherwise the leading program is implicit and prepended.
        // Only the first line drives this — continuation lines (`map`, `diff`)
        // legitimately mention `{prog}` while still wanting the prepend.
        let first_line = snippet.split('\n').next().unwrap_or("");
        let self_describing = first_line.contains(PROG);
        if !self_describing {
            out.push_str(&prog);
        }
        let snippet = snippet.replace(PROG, &prog);
        if !snippet.is_empty() {
            if !self_describing {
                out.push(' ');
            }
            out.push_str(&snippet);
        }
        if !comment.is_empty() {
            let comment = comment.replace(PROG, &prog);
            out.push_str("\n  # ");
            out.push_str(&comment);
        }
    }
    out
}
