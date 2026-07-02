//! `bl uninstall` — remove browserlane cleanly.
//!
//! Default: stop the daemon, remove the install directory (the binary and its
//! parent `.browserlane` dir), and strip the PATH entry the installer added.
//! `--purge` also removes the Chrome-for-Testing cache and the screenshots dir
//! (user data, so it's opt-in). Leaves no dangling PATH line behind.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::anyhow;

/// A path targeted for removal, with a human label for the summary.
struct Target {
    label: &'static str,
    path: PathBuf,
}

/// Removes the browserlane block from a shell rc file (the `# browserlane`
/// comment + the following `export PATH=...<install_dir>...` line the installer
/// appended). Idempotent; silently does nothing if not present.
fn strip_shell_path(rc: &Path, install_dir: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(rc) else {
        return false;
    };
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut removed = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Installer wrote: "# browserlane" then the export line.
        if line.trim() == "# browserlane"
            && lines.get(i + 1).is_some_and(|n| n.contains(install_dir))
        {
            i += 2;
            removed = true;
            continue;
        }
        // Defensively also drop a bare export line if the comment drifted.
        if line.contains(install_dir) && line.contains("PATH") {
            i += 1;
            removed = true;
            continue;
        }
        out.push(line);
        i += 1;
    }
    if !removed {
        return false;
    }
    // Trim a trailing blank left behind, preserve a final newline.
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    let mut body = out.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    std::fs::write(rc, body).is_ok()
}

/// Removes `install_dir` from the Windows user PATH env var.
#[cfg(windows)]
fn strip_windows_path(install_dir: &str) -> bool {
    use std::process::Command;
    // Read + rewrite the User PATH via PowerShell (same registry scope the
    // installer wrote). Kept as a shell-out to avoid a winreg dependency.
    let ps = format!(
        "$p=[Environment]::GetEnvironmentVariable('Path','User'); \
         $n=($p -split ';' | Where-Object {{ $_ -ne '{dir}' -and $_ -ne '' }}) -join ';'; \
         [Environment]::SetEnvironmentVariable('Path',$n,'User')",
        dir = install_dir.replace('\'', "''")
    );
    Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn confirm(prompt: &str) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("{prompt} [y/N] ");
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Runs `bl uninstall`. `purge` also removes the Chrome cache + screenshots;
/// `yes` skips the confirmation prompt.
pub async fn run(purge: bool, yes: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe()
        .map_err(|e| anyhow!("cannot resolve the running executable: {e}"))?;
    let bin_dir = exe
        .parent()
        .ok_or_else(|| anyhow!("cannot resolve the install directory"))?
        .to_path_buf();

    // The dir to remove: the binary's dir, plus its parent when that parent is
    // the installer's `.browserlane` root (so `.browserlane/bin` -> remove
    // `.browserlane`). A custom BL_INSTALL_DIR removes only the bin dir.
    let install_root = match bin_dir.parent() {
        Some(parent) if parent.file_name().and_then(|n| n.to_str()) == Some(".browserlane") => {
            parent.to_path_buf()
        }
        _ => bin_dir.clone(),
    };

    let mut targets = vec![Target { label: "install directory", path: install_root.clone() }];
    if purge {
        if let Ok(cache) = crate::paths::get_cache_dir() {
            targets.push(Target { label: "Chrome cache", path: cache });
        }
        if let Ok(shots) = crate::paths::get_screenshot_dir() {
            targets.push(Target { label: "screenshots", path: shots });
        }
    }

    println!("bl uninstall will remove:");
    for t in &targets {
        let note = if t.path.exists() { "" } else { "  (not present)" };
        println!("  - {:<18} {}{note}", t.label, t.path.display());
    }
    println!("  - PATH entry the installer added");
    if !purge {
        println!("\nChrome cache and screenshots are kept — re-run with --purge to remove them too.");
    }

    if !yes && !confirm("\nProceed?") {
        if std::io::stdin().is_terminal() {
            println!("Aborted.");
            return Ok(());
        }
        return Err(anyhow!(
            "refusing to uninstall without confirmation — pass --yes to proceed non-interactively"
        ));
    }

    // Stop the daemon so it isn't left running against a removed binary.
    if crate::daemon::shutdown().await.is_ok() {
        eprintln!("stopped the running daemon");
    }

    // Strip PATH before deleting the dir (so a failure here still leaves the
    // binary findable).
    let install_dir_str = bin_dir.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        if strip_windows_path(&install_dir_str) {
            eprintln!("removed {install_dir_str} from your user PATH — restart your terminal");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = crate::paths::user_home_dir() {
            let mut stripped = false;
            for rc in [".zshrc", ".bashrc", ".profile"] {
                if strip_shell_path(&home.join(rc), &install_dir_str) {
                    eprintln!("removed the PATH entry from ~/{rc}");
                    stripped = true;
                }
            }
            if !stripped {
                eprintln!("note: no PATH entry found in ~/.zshrc, ~/.bashrc, or ~/.profile");
            }
        }
    }

    // Remove targets. On unix the running binary can be unlinked while
    // executing (the inode survives until exit), so removing install_root works
    // even though we're running from inside it.
    let mut failures = Vec::new();
    for t in &targets {
        if !t.path.exists() {
            continue;
        }
        if let Err(e) = std::fs::remove_dir_all(&t.path) {
            // Windows can't delete the running bl.exe; report it honestly.
            failures.push(format!("{} ({}): {e}", t.label, t.path.display()));
        } else {
            eprintln!("removed {}", t.path.display());
        }
    }

    if !failures.is_empty() {
        eprintln!("\nSome paths could not be removed:");
        for f in &failures {
            eprintln!("  - {f}");
        }
        #[cfg(windows)]
        eprintln!(
            "  (Windows can't delete bl.exe while it's running — delete {} after this process exits.)",
            install_root.display()
        );
        return Err(anyhow!("uninstall completed with errors"));
    }

    println!("browserlane uninstalled. Thanks for trying it.");
    Ok(())
}
