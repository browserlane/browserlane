//! browserlane-specific CLI commands and MCP tools.
//!
//! This module is NOT part of the ported core. Everything here is
//! browserlane-original. The port (in `src/`) calls into here via 4
//! well-defined seams (each tagged `// ext-seam`). To add a feature, put it
//! in [`cli`] (CLI subcommands) or [`mcp`] (MCP tools) — never in `src/`.

use clap::{ArgMatches, Command};
use serde_json::{Map, Value};

use crate::agent::{Tool, ToolsCallResult};

pub mod cli;
pub mod mcp;
mod add_mcp;

/// ext-seam: extend the CLI command tree with browserlane-specific subcommands.
pub fn register_cli(cli: Command) -> Command {
    cli::register(cli)
}

/// ext-seam: dispatch a browserlane-specific CLI subcommand. Returns true if
/// `name` was handled; false otherwise (so the core can fall back to default
/// behavior, e.g. printing root help).
pub async fn dispatch_cli(
    name: &str,
    sub: &ArgMatches,
    headless: bool,
    json_output: bool,
) -> bool {
    cli::dispatch(name, sub, headless, json_output).await
}

/// ext-seam: extend the MCP tool catalog with browserlane-specific tools.
pub fn register_mcp_tools(tools: &mut Vec<Tool>) {
    mcp::register(tools);
}

/// ext-seam: dispatch a browserlane-specific MCP tool. Returns `Some(result)`
/// if `name` was handled; `None` otherwise (so the core can return its faithful
/// `unknown tool` error).
pub async fn dispatch_mcp_tool(
    name: &str,
    args: Map<String, Value>,
) -> Option<anyhow::Result<ToolsCallResult>> {
    mcp::dispatch(name, args).await
}
