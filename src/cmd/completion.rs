//! Shell-completion command. cobra auto-registers `completion` (bash/zsh/fish/
//! powershell) with no dedicated Go source file, so this has no `go-rev` pin;
//! browserlane provides the equivalent via `clap_complete`. Per the PO decision
//! the generated scripts need not byte-match cobra's — only the command's
//! existence, the four shells, and the exit codes/control flow must match.

use clap::{Arg, Command};
use clap_complete::{generate, Shell};

use super::diagnostics::prog_name;
use super::examples::examples;

/// The four shells `run_completion` knows how to emit scripts for. Surfaced in
/// the command's description and in the positional's help.
const SHELLS: [&str; 4] = ["bash", "zsh", "fish", "powershell"];

/// `completion [shell]` — `shell` is an optional positional (bash/zsh/fish/
/// powershell). A missing or unrecognized shell prints the command's help with
/// exit 0 (see `main`'s dispatch), mirroring cobra.
pub fn completion_command() -> Command {
    // The long description names the program inline; like examples() we splice
    // the live prog_name() at build time rather than leaving a render-time
    // sentinel.
    //
    // Two deliberate shape choices worth calling out:
    //   * No bash/zsh/fish/powershell *subcommands*. This binary takes `shell`
    //     as a positional (dispatched by run_completion), so there are no
    //     subcommands to list — the valid shells are named in long_about and the
    //     positional's help instead.
    //   * The positional is intentionally left unconstrained (no possible-value
    //     parser): a missing/unrecognized shell must fall through to
    //     run_completion -> false -> print help with exit 0 (see main's
    //     dispatch), so constraining it (which would make clap reject unknown
    //     shells with exit 2) would change that documented exit behavior.
    Command::new("completion")
        .about("Generate the autocompletion script for the specified shell")
        .long_about(format!(
            "Generate the autocompletion script for {prog} for the specified shell.\n\
             Run with one of: {shells}.\n\
             See the examples below for how to load the generated script.",
            prog = prog_name(),
            shells = SHELLS.join(", "),
        ))
        .arg(
            Arg::new("shell")
                .num_args(0..=1)
                .help(format!("Shell to generate the script for ({})", SHELLS.join(", "))),
        )
        .after_help(examples(&[
            (
                "source <({prog} completion bash)",
                "bash: load completions in the current shell",
            ),
            (
                "{prog} completion fish | source",
                "fish: load completions in the current shell",
            ),
            (
                "source <({prog} completion zsh)",
                "zsh: load completions in the current shell (enable once with: echo \"autoload -U compinit; compinit\" >> ~/.zshrc)",
            ),
            (
                "{prog} completion powershell | Out-String | Invoke-Expression",
                "powershell: load completions in the current shell",
            ),
        ]))
}

/// Writes the completion script for `shell` to stdout, using the full CLI
/// definition `cli`. Returns `false` when `shell` is missing or not one of the
/// four supported shells, so the caller prints the completion help (exit 0),
/// like cobra's `completion` / `completion <bogus>`.
pub fn run_completion(shell: Option<&str>, mut cli: Command) -> bool {
    let sh = match shell {
        Some("bash") => Shell::Bash,
        Some("zsh") => Shell::Zsh,
        Some("fish") => Shell::Fish,
        Some("powershell") => Shell::PowerShell,
        _ => return false,
    };
    let bin = cli.get_name().to_string();
    generate(sh, &mut cli, bin, &mut std::io::stdout());
    true
}
