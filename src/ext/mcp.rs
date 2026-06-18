//! browserlane-specific MCP tools.
//!
//! Register tool schemas in [`register`] and dispatch them in [`dispatch`].
//! There are no browserlane-specific MCP tools yet — the stubs are here so
//! the ext seam is exercised at compile time.

use serde_json::{Map, Value};

use crate::agent::{Tool, ToolsCallResult};

/// Register all browserlane-specific MCP tools on the catalog.
pub fn register(_tools: &mut Vec<Tool>) {
    // No browserlane MCP tools yet.
}

/// Dispatch a browserlane-specific MCP tool. Returns `Some(result)` if
/// handled; `None` otherwise.
pub async fn dispatch(
    _name: &str,
    _args: Map<String, Value>,
) -> Option<anyhow::Result<ToolsCallResult>> {
    None
}
