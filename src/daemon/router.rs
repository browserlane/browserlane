use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use super::Conn;

use super::daemon::Daemon;
use crate::agent;
use crate::errors::format_go_duration;
use crate::log;

/// Returned by daemon/status.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatusResult {
    pub version: String,
    pub pid: i32,
    pub uptime: String,
    pub socket: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
}

impl Daemon {
    /// Processes a single client connection: one JSON-RPC request, one response.
    pub(crate) async fn handle_connection(self: Arc<Self>, conn: Conn) {
        self.touch_activity();

        let (read_half, mut write_half) = tokio::io::split(conn);
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        // Read deadline (60s).
        match tokio::time::timeout(Duration::from_secs(60), reader.read_line(&mut line)).await {
            Ok(Ok(n)) if n > 0 => {}
            _ => return,
        }

        let response = match self.handle_request(line.trim_end().as_bytes()).await {
            Some(r) => r,
            None => return,
        };

        let data = match serde_json::to_string(&response) {
            Ok(d) => d,
            Err(e) => {
                log::debug(&format!("marshal error: {e}"));
                return;
            }
        };

        let _ = tokio::time::timeout(Duration::from_secs(60), async {
            let _ = write_half.write_all(data.as_bytes()).await;
            let _ = write_half.write_all(b"\n").await;
            let _ = write_half.flush().await;
        })
        .await;
    }

    /// Parses and routes a JSON-RPC request.
    #[allow(clippy::question_mark)]
    async fn handle_request(&self, data: &[u8]) -> Option<agent::Response> {
        let req: agent::Request = match serde_json::from_slice(data) {
            Ok(r) => r,
            Err(e) => {
                return Some(agent::Response {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(agent::Error {
                        code: agent::PARSE_ERROR,
                        message: "Parse error".to_string(),
                        data: Some(json!(e.to_string())),
                    }),
                });
            }
        };

        if req.jsonrpc != "2.0" {
            return Some(agent::Response {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: None,
                error: Some(agent::Error {
                    code: agent::INVALID_REQUEST,
                    message: "Invalid Request".to_string(),
                    data: Some(json!("jsonrpc must be '2.0'")),
                }),
            });
        }

        let (result, mcp_err) = self.route(&req).await;

        if req.id.is_none() {
            return None;
        }

        if let Some(err) = mcp_err {
            return Some(agent::Response {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: None,
                error: Some(err),
            });
        }

        Some(agent::Response {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result,
            error: None,
        })
    }

    /// Dispatches requests to the appropriate handler.
    async fn route(&self, req: &agent::Request) -> (Option<Value>, Option<agent::Error>) {
        log::debug(&format!("daemon request: {}", req.method));

        match req.method.as_str() {
            "daemon/status" => self.handle_status(),
            "daemon/shutdown" => {
                self.shutdown(); // shutdown asynchronously so we can send the response
                (Some(json!({ "status": "shutting down" })), None)
            }
            "tools/call" => self.handle_tools_call(req.params.clone()).await,
            "tools/list" => (
                serde_json::to_value(agent::ToolsListResult {
                    tools: agent::get_tool_schemas(),
                })
                .ok(),
                None,
            ),
            "initialize" => self.handle_initialize(),
            "initialized" | "notifications/initialized" => (None, None),
            _ => (
                None,
                Some(agent::Error {
                    code: agent::METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: Some(json!(req.method)),
                }),
            ),
        }
    }

    /// Returns daemon status information.
    fn handle_status(&self) -> (Option<Value>, Option<agent::Error>) {
        let uptime = Duration::from_secs(self.start_time.elapsed().as_secs());
        let _ = self.start_time_sys; // RFC3339 startTime is wired in Phase 5 (not surfaced by the status command)
        let result = StatusResult {
            version: self.version.clone(),
            pid: std::process::id() as i32,
            uptime: format_go_duration(uptime),
            socket: self.socket_path.lock().unwrap().clone(),
            start_time: String::new(),
        };
        (serde_json::to_value(result).ok(), None)
    }

    /// Handles the MCP initialize request.
    fn handle_initialize(&self) -> (Option<Value>, Option<agent::Error>) {
        let result = agent::InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: agent::ServerCapabilities {
                tools: Some(agent::ToolsCapability { list_changed: false }),
            },
            server_info: agent::ServerInfo {
                name: "browserlane".to_string(),
                version: self.version.clone(),
            },
        };
        (serde_json::to_value(result).ok(), None)
    }

    /// Executes a tool and returns the result.
    async fn handle_tools_call(&self, params: Option<Value>) -> (Option<Value>, Option<agent::Error>) {
        let p: agent::ToolsCallParams = match serde_json::from_value(params.unwrap_or(Value::Null)) {
            Ok(p) => p,
            Err(e) => {
                return (
                    None,
                    Some(agent::Error {
                        code: agent::INVALID_PARAMS,
                        message: "Invalid params".to_string(),
                        data: Some(json!(e.to_string())),
                    }),
                );
            }
        };

        // Serialize handler access — handlers are not thread-safe.
        let result = self.handlers.lock().await.call(&p.name, p.arguments).await;

        match result {
            Err(e) => (
                serde_json::to_value(agent::ToolsCallResult {
                    content: vec![agent::Content {
                        content_type: "text".to_string(),
                        text: e.to_string(),
                        data: String::new(),
                        mime_type: String::new(),
                    }],
                    is_error: true,
                })
                .ok(),
                None,
            ),
            Ok(r) => (serde_json::to_value(r).ok(), None),
        }
    }
}
