//! The JSON-RPC 2.0 / MCP type definitions plus the MCP stdio `Server`: a
//! JSON-RPC 2.0 server over stdin/stdout for LLM agents.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_tungstenite::tungstenite::http::HeaderMap;

use super::handlers::{new_handlers, Handlers};

/// JSON-RPC 2.0 request structure.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response structure.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

/// JSON-RPC 2.0 error structure.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// Standard JSON-RPC error codes.
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(default, rename = "listChanged", skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub arguments: serde_json::Map<String, Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolsCallResult {
    pub content: Vec<Content>,
    #[serde(default, rename = "isError", skip_serializing_if = "is_false")]
    pub is_error: bool,
}

/// A single MCP content block. Mirrors Go's custom MarshalJSON: a text block
/// always carries a (possibly empty) "text" field; an image block carries
/// "data" and "mimeType".
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub data: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.content_type.as_str() {
            "image" => {
                let mut s = serializer.serialize_struct("Content", 3)?;
                s.serialize_field("type", &self.content_type)?;
                s.serialize_field("data", &self.data)?;
                s.serialize_field("mimeType", &self.mime_type)?;
                s.end()
            }
            _ => {
                let mut s = serializer.serialize_struct("Content", 2)?;
                s.serialize_field("type", &self.content_type)?;
                s.serialize_field("text", &self.text)?;
                s.end()
            }
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

// ---------------------------------------------------------------------------
// MCP stdio server.
// ---------------------------------------------------------------------------

/// `initialize` request params. Parsed only to validate shape (the result is
/// fixed); mirrors Go's InitializeParams. All fields optional so missing ones
/// decode to zero values, matching Go's json.Unmarshal into a struct.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct InitializeParams {
    #[serde(default, rename = "protocolVersion")]
    protocol_version: String,
    #[serde(default)]
    capabilities: ClientCapabilities,
    #[serde(default, rename = "clientInfo")]
    client_info: ClientInfo,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct ClientCapabilities {
    #[serde(default)]
    roots: Option<RootsCapability>,
    #[serde(default)]
    sampling: Option<SamplingCapability>,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct RootsCapability {
    #[serde(default, rename = "listChanged")]
    list_changed: bool,
}

#[derive(Debug, Default, Deserialize)]
struct SamplingCapability {}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct ClientInfo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
}

/// Configures the MCP server.
pub struct ServerOptions {
    /// Directory for saving screenshots (empty = disabled).
    pub screenshot_dir: String,
    /// Remote BiDi WebSocket URL (empty = local browser).
    pub connect_url: String,
    /// Headers for the remote WebSocket connection.
    pub connect_headers: Option<HeaderMap>,
}

/// The MCP server: handles JSON-RPC 2.0 over stdio.
pub struct Server {
    handlers: Handlers,
    version: String,
}

/// Creates a new MCP server.
pub fn new_server(version: &str, opts: ServerOptions) -> Server {
    Server {
        handlers: new_handlers(&opts.screenshot_dir, false, &opts.connect_url, opts.connect_headers),
        version: version.to_string(),
    }
}

impl Server {
    /// Runs the server loop, reading requests from stdin and writing responses to
    /// stdout until EOF.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| anyhow::anyhow!("read error: {e}"))?;
            if n == 0 {
                return Ok(()); // EOF, clean exit
            }
            // Go's ReadBytes('\n') returns io.EOF (and we exit) when the stream
            // ends without a trailing newline, dropping the partial line.
            if !line.ends_with('\n') {
                return Ok(());
            }
            // Skip empty lines (Go: len(line) <= 1, where line includes '\n').
            if line.len() <= 1 {
                continue;
            }

            if let Some(resp) = self.handle_request(line.as_bytes()).await {
                let data = serde_json::to_string(&resp).map_err(|e| anyhow::anyhow!("write error: {e}"))?;
                stdout
                    .write_all(data.as_bytes())
                    .await
                    .map_err(|e| anyhow::anyhow!("write error: {e}"))?;
                stdout
                    .write_all(b"\n")
                    .await
                    .map_err(|e| anyhow::anyhow!("write error: {e}"))?;
                stdout.flush().await.map_err(|e| anyhow::anyhow!("write error: {e}"))?;
            }
        }
    }

    /// Parses and routes a JSON-RPC request, returning the response (or None for
    /// notifications, which get no reply).
    async fn handle_request(&mut self, data: &[u8]) -> Option<Response> {
        let req: Request = match serde_json::from_slice(data) {
            Ok(r) => r,
            Err(e) => {
                return Some(Response {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(Error {
                        code: PARSE_ERROR,
                        message: "Parse error".to_string(),
                        data: Some(json!(e.to_string())),
                    }),
                });
            }
        };

        // Validate JSON-RPC version.
        if req.jsonrpc != "2.0" {
            return Some(Response {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: None,
                error: Some(Error {
                    code: INVALID_REQUEST,
                    message: "Invalid Request".to_string(),
                    data: Some(json!("jsonrpc must be '2.0'")),
                }),
            });
        }

        let (result, error) = self.route(&req).await;

        // Notifications (no ID) don't get a response (even on error).
        req.id.as_ref()?;

        if let Some(error) = error {
            return Some(Response {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(error),
            });
        }

        Some(Response {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result,
            error: None,
        })
    }

    /// Dispatches requests to the appropriate handler.
    async fn route(&mut self, req: &Request) -> (Option<Value>, Option<Error>) {
        crate::log::debug(&format!("mcp request: {}", req.method));

        match req.method.as_str() {
            "initialize" => self.handle_initialize(req.params.as_ref()),
            "initialized" | "notifications/initialized" => (None, None),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(req.params.clone()).await,
            _ => (
                None,
                Some(Error {
                    code: METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: Some(json!(req.method)),
                }),
            ),
        }
    }

    /// Handles the `initialize` request.
    fn handle_initialize(&self, params: Option<&Value>) -> (Option<Value>, Option<Error>) {
        if let Some(p) = params {
            if let Err(e) = serde_json::from_value::<InitializeParams>(p.clone()) {
                return (
                    None,
                    Some(Error {
                        code: INVALID_PARAMS,
                        message: "Invalid params".to_string(),
                        data: Some(json!(e.to_string())),
                    }),
                );
            }
        }

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: false }),
            },
            server_info: ServerInfo {
                name: "browserlane".to_string(),
                version: self.version.clone(),
            },
        };
        (serde_json::to_value(result).ok(), None)
    }

    /// Returns the list of available tools.
    fn handle_tools_list(&self) -> (Option<Value>, Option<Error>) {
        (
            serde_json::to_value(ToolsListResult {
                tools: super::schema::get_tool_schemas(),
            })
            .ok(),
            None,
        )
    }

    /// Executes a tool and returns the result.
    async fn handle_tools_call(&mut self, params: Option<Value>) -> (Option<Value>, Option<Error>) {
        let p: ToolsCallParams = match serde_json::from_value(params.unwrap_or(Value::Null)) {
            Ok(p) => p,
            Err(e) => {
                return (
                    None,
                    Some(Error {
                        code: INVALID_PARAMS,
                        message: "Invalid params".to_string(),
                        data: Some(json!(e.to_string())),
                    }),
                );
            }
        };

        match self.handlers.call(&p.name, p.arguments).await {
            Ok(result) => (serde_json::to_value(result).ok(), None),
            Err(e) => (
                serde_json::to_value(ToolsCallResult {
                    content: vec![Content {
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
        }
    }

    /// Cleans up the server resources (closes the browser session).
    pub async fn close(&mut self) {
        self.handlers.close().await;
    }
}
