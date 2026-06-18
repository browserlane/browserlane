use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Value};

use super::connection::Connection;
use super::protocol::{new_command, unmarshal_message, Message};
use crate::errors::format_go_duration;

/// Maximum time to wait for a BiDi command response.
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Callback for BiDi events received while waiting for command responses.
type EventHandler = Arc<dyn Fn(String) + Send + Sync>;

/// A BiDi client that wraps a WebSocket connection.
pub struct Client {
    conn: Arc<Connection>,
    verbose: bool,
    /// Optional callback for BiDi events received while waiting for responses.
    /// Interior-mutable so it can be (re)set through a shared `Arc<Client>` (the
    /// MCP recorder installs it via `set_event_handler` after launch).
    event_handler: std::sync::Mutex<Option<EventHandler>>,
}

impl Client {
    /// Creates a new BiDi client from a WebSocket connection.
    pub fn new(conn: Arc<Connection>) -> Self {
        Client {
            conn,
            verbose: false,
            event_handler: std::sync::Mutex::new(None),
        }
    }

    /// Enables or disables verbose logging of JSON messages.
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Sets a callback for BiDi events received while waiting for command
    /// responses. Pass `None` to stop forwarding events.
    pub fn set_event_handler(&self, handler: Option<EventHandler>) {
        *self.event_handler.lock().unwrap() = handler;
    }

    /// Sends a BiDi command and waits for the response (60s timeout).
    pub async fn send_command(&self, method: &str, params: Value) -> anyhow::Result<Message> {
        self.send_command_with_timeout(method, params, DEFAULT_COMMAND_TIMEOUT)
            .await
    }

    /// Sends a BiDi command and waits for the response with a custom timeout.
    pub async fn send_command_with_timeout(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> anyhow::Result<Message> {
        let cmd = new_command(method, Some(params));

        let data = cmd
            .marshal()
            .map_err(|e| anyhow!("failed to marshal command: {e}"))?;
        let data = String::from_utf8_lossy(&data).into_owned();

        if self.verbose {
            println!("       --> {data}");
        }

        self.conn
            .send(&data)
            .await
            .map_err(|e| anyhow!("failed to send command: {e}"))?;

        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() > deadline {
                return Err(anyhow!(
                    "timeout waiting for response to {method} after {}",
                    format_go_duration(timeout)
                ));
            }

            let resp = self
                .conn
                .receive()
                .await
                .map_err(|e| anyhow!("failed to receive response: {e}"))?;

            if self.verbose {
                println!("       <-- {resp}");
            }

            let msg = unmarshal_message(resp.as_bytes())
                .map_err(|e| anyhow!("failed to parse response: {e}"))?;

            // Is this the response we're waiting for?
            if msg.id == Some(cmd.id) {
                if msg.is_error() {
                    if let Some(err_data) = msg.get_error()? {
                        return Err(anyhow!(
                            "BiDi error: {} - {}",
                            err_data.error,
                            err_data.message
                        ));
                    }
                    let raw = msg
                        .error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_default();
                    return Err(anyhow!("BiDi error: {raw}"));
                }
                return Ok(msg);
            }

            // If it's an event, forward to the handler if set, otherwise skip.
            if msg.is_event() {
                if self.verbose {
                    println!("       (event, skipping)");
                }
                let handler = self.event_handler.lock().unwrap().clone();
                if let Some(handler) = handler {
                    handler(resp);
                }
                continue;
            }
        }
    }

    /// Sends a `session.status` command and returns the result.
    pub async fn session_status(&self) -> anyhow::Result<SessionStatusResult> {
        let msg = self.send_command("session.status", json!({})).await?;
        let result = msg.result.unwrap_or(Value::Null);
        serde_json::from_value(result)
            .map_err(|e| anyhow!("failed to parse session.status result: {e}"))
    }

    /// Sends a `session.new` command and returns the result.
    pub async fn session_new(&self, capabilities: Value) -> anyhow::Result<SessionNewResult> {
        let params = json!({ "capabilities": capabilities });
        let msg = self.send_command("session.new", params).await?;
        let result = msg.result.unwrap_or(Value::Null);
        serde_json::from_value(result)
            .map_err(|e| anyhow!("failed to parse session.new result: {e}"))
    }

    /// Closes the underlying connection.
    pub async fn close(&self) -> anyhow::Result<()> {
        self.conn.close().await
    }
}

/// Result of the `session.status` command.
#[derive(Debug, Deserialize)]
pub struct SessionStatusResult {
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub message: String,
}

/// Result of the `session.new` command.
#[derive(Debug, Deserialize)]
pub struct SessionNewResult {
    #[serde(rename = "sessionId", default)]
    pub session_id: String,
    #[serde(default)]
    pub capabilities: serde_json::Map<String, Value>,
}
