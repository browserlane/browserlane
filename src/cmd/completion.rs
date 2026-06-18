//! Shell-completion command. cobra auto-registers `completion` (bash/zsh/fish/
//! powershell) with no dedicated Go source file, so this has no `go-rev` pin;
//! browserlane provides the equivalent via `clap_complete`. Per the PO decision
//! the generated scripts need not byte-match cobra's — only the command's
//! existence, the four shells, and the exit codes/control flow must match.

use clap::{Arg, Command};
use clap_complete::{generate, Shell};

/// `completion [shell]` — `shell` is an optional positional (bash/zsh/fish/
/// powershell). A missing or unrecognized shell prints the command's help with
/// exit 0 (see `main`'s dispatch), mirroring cobra.
pub fn completion_command() -> Command {
    Command::new("completion")
        .about("Generate the autocompletion script for the specified shell")
        .arg(Arg::new("shell").num_args(0..=1))
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
