use clap::{Arg, ArgAction, ArgMatches, Command};

use super::examples::examples;

/// The browser skill, embedded at build time. Mirrors Go's `//go:embed SKILL.md`.
/// (SKILL.md is browserlane-rebranded; the skill installs to
/// `~/.claude/skills/browserlane/`.)
const SKILL_MD: &str = include_str!("SKILL.md");

pub fn skill_command() -> Command {
    Command::new("add-skill")
        .about("Install browserlane browser skill for Claude Code")
        .arg(
            Arg::new("stdout")
                .long("stdout")
                .action(ArgAction::SetTrue)
                .help("Print skill content to stdout instead of installing"),
        )
        .after_help(examples(&[
            ("add-skill", "Installs skill to ~/.claude/skills/vibe-check/"),
            ("add-skill --stdout", "Print skill content to stdout"),
        ]))
}

pub fn run_skill(matches: &ArgMatches) {
    if matches.get_flag("stdout") {
        print!("{SKILL_MD}");
        return;
    }
    if let Err(e) = install_skill() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn install_skill() -> Result<(), String> {
    let home =
        crate::paths::user_home_dir().map_err(|e| format!("could not find home directory: {e}"))?;

    let skill_dir = home.join(".claude").join("skills").join("browserlane");
    std::fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("could not create skill directory: {e}"))?;

    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, SKILL_MD).map_err(|e| format!("could not write SKILL.md: {e}"))?;

    println!("Installed browserlane skill to {}", skill_dir.display());
    println!("Files:");
    println!("  {}", skill_path.display());
    Ok(())
}
