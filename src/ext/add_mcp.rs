//! `bl add-mcp <client>` — register the browserlane MCP server with a coding
//! agent (Claude Code, Claude Desktop, Cursor, VS Code, Codex).
//!
//! browserlane-original (not part of the ported core). Each client stores its
//! MCP config differently (JSON under `mcpServers`, JSON under `servers`, or
//! TOML `[mcp_servers.*]`), so there's a small adapter per client. Every entry
//! points at THIS binary's own absolute path (`current_exe`) so the server
//! launches regardless of whether `bl` is on `PATH`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// Prints the supported clients.
pub fn list() {
    println!("Register the browserlane MCP server with a coding agent:\n");
    println!("  bl add-mcp <client> [--stdout]\n");
    println!("Clients:");
    println!("  claude          Claude Code      (runs `claude mcp add`)");
    println!("  claude-desktop  Claude Desktop   (claude_desktop_config.json)");
    println!("  cursor          Cursor           (~/.cursor/mcp.json)");
    println!("  vscode          VS Code          (.vscode/mcp.json, current project)");
    println!("  codex           OpenAI Codex CLI (~/.codex/config.toml)\n");
    println!("--stdout prints the config snippet instead of writing the file.");
}

/// Registers browserlane with `client`. With `stdout`, prints the snippet
/// instead of writing the config file.
pub fn add(client: &str, stdout: bool) -> Result<(), String> {
    let bl = current_bl()?;
    match client {
        "claude" | "claude-code" => add_claude_code(&bl, stdout),
        "claude-desktop" => upsert_json(
            &claude_desktop_path()?,
            "mcpServers",
            entry(&bl, false),
            stdout,
            "Claude Desktop",
        ),
        "cursor" => upsert_json(
            &home()?.join(".cursor").join("mcp.json"),
            "mcpServers",
            entry(&bl, false),
            stdout,
            "Cursor",
        ),
        "vscode" | "vs-code" => upsert_json(
            &PathBuf::from(".vscode").join("mcp.json"),
            "servers",
            entry(&bl, true),
            stdout,
            "VS Code",
        ),
        "codex" => upsert_codex(&home()?.join(".codex").join("config.toml"), &bl, stdout),
        other => Err(format!(
            "unknown client {other:?}. Run `bl add-mcp --list` for the supported clients."
        )),
    }
}

/// The browserlane server entry. VS Code's schema needs an explicit `type`.
fn entry(bl: &str, vscode: bool) -> Value {
    if vscode {
        json!({ "type": "stdio", "command": bl, "args": ["mcp"] })
    } else {
        json!({ "command": bl, "args": ["mcp"] })
    }
}

fn current_bl() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("could not resolve the bl binary path: {e}"))
}

fn home() -> Result<PathBuf, String> {
    crate::paths::user_home_dir().map_err(|e| format!("could not find home directory: {e}"))
}

fn claude_desktop_path() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var_os("APPDATA").ok_or_else(|| "APPDATA is not set".to_string())?;
        Ok(PathBuf::from(appdata)
            .join("Claude")
            .join("claude_desktop_config.json"))
    } else if cfg!(target_os = "macos") {
        Ok(home()?
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json"))
    } else {
        Ok(home()?
            .join(".config")
            .join("Claude")
            .join("claude_desktop_config.json"))
    }
}

/// Claude Code owns its own config — shell out to its CLI.
fn add_claude_code(bl: &str, stdout: bool) -> Result<(), String> {
    let manual = format!("claude mcp add browserlane -- {bl} mcp");
    if stdout {
        println!("{manual}");
        return Ok(());
    }
    match std::process::Command::new("claude")
        .args(["mcp", "add", "browserlane", "--", bl, "mcp"])
        .status()
    {
        Ok(s) if s.success() => {
            println!("✓ Registered browserlane with Claude Code.");
            Ok(())
        }
        Ok(s) => Err(format!("`claude mcp add` failed ({s}). Run it yourself:\n  {manual}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(format!(
            "`claude` CLI not found on PATH. Once it's installed, run:\n  {manual}"
        )),
        Err(e) => Err(format!("could not run `claude`: {e}")),
    }
}

/// Reads an existing JSON config (or starts fresh), inserts the browserlane
/// server under `top_key`, and writes it back — preserving any other servers.
fn upsert_json(
    path: &Path,
    top_key: &str,
    server: Value,
    stdout: bool,
    client: &str,
) -> Result<(), String> {
    if stdout {
        let snippet = json!({ top_key: { "browserlane": server } });
        println!("{}", serde_json::to_string_pretty(&snippet).unwrap_or_default());
        return Ok(());
    }
    let mut root: Value = if path.exists() {
        let txt = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        if txt.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&txt).map_err(|e| format!("parse {}: {e}", path.display()))?
        }
    } else {
        json!({})
    };
    let obj = root
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;
    let servers = obj.entry(top_key).or_insert_with(|| json!({}));
    servers
        .as_object_mut()
        .ok_or_else(|| format!("`{top_key}` in {} is not an object", path.display()))?
        .insert("browserlane".to_string(), server);
    write_file(path, serde_json::to_string_pretty(&root).map_err(|e| e.to_string())? + "\n")?;
    report(client, path);
    Ok(())
}

/// Codex uses TOML; edit it format-preservingly so existing settings/comments
/// survive.
fn upsert_codex(path: &Path, bl: &str, stdout: bool) -> Result<(), String> {
    if stdout {
        println!("# ~/.codex/config.toml");
        println!("[mcp_servers.browserlane]");
        println!("command = \"{bl}\"");
        println!("args = [\"mcp\"]");
        return Ok(());
    }
    let mut doc = if path.exists() {
        std::fs::read_to_string(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("parse {}: {e}", path.display()))?
    } else {
        toml_edit::DocumentMut::new()
    };
    if !doc.contains_key("mcp_servers") {
        let mut parent = toml_edit::Table::new();
        parent.set_implicit(true); // don't emit a bare `[mcp_servers]` header
        doc.insert("mcp_servers", toml_edit::Item::Table(parent));
    }
    let servers = doc["mcp_servers"]
        .as_table_mut()
        .ok_or_else(|| format!("`mcp_servers` in {} is not a table", path.display()))?;
    let mut tbl = toml_edit::Table::new();
    tbl["command"] = toml_edit::value(bl);
    let mut args = toml_edit::Array::new();
    args.push("mcp");
    tbl["args"] = toml_edit::value(args);
    servers.insert("browserlane", toml_edit::Item::Table(tbl));
    write_file(path, doc.to_string())?;
    report("Codex", path);
    Ok(())
}

fn write_file(path: &Path, contents: String) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
    }
    std::fs::write(path, contents).map_err(|e| format!("write {}: {e}", path.display()))
}

fn report(client: &str, path: &Path) {
    println!("✓ Registered browserlane with {client}");
    println!("  {}", path.display());
    println!("  Restart {client} to load it.");
}
