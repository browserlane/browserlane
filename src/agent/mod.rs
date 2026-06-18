// Package plumbing for clicker/internal/agent (MCP server).
// Phase 2 ports the JSON-RPC/MCP types + a minimal Handlers (navigate slice);
// the MCP stdio server and full tool surface are ported in Phase 4.

mod handlers;
mod schema;
mod server;

#[allow(unused_imports)]
pub use handlers::*;
#[allow(unused_imports)]
pub use schema::*;
#[allow(unused_imports)]
pub use server::*;
